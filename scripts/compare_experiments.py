#!/usr/bin/env python3
"""
ZEBRA Cross-zkVM Comparison Experiment
=======================================
Runs verification strategies for every opcode of every chip across all
supported zkVMs using single-point search (--range-interval 0).

Standard mode  (default):
  bb           — pure branch-and-bound
  bb_blocking  — branch-and-bound with blocking closure (--blocking-closure)
  z3           — SMT solver (z3)

Ablation study mode  (--ablation):
  bb                — full bb (baseline)
  bb_no_heuristic   — bb without priority-queue heuristic (pure DFS)
  bb_no_simplify    — bb without constraint simplification
  bb_no_refinement  — bb without interval refinement

Early-stop rule:  if ANY single trial for an opcode fails (timeout or
verification failure), that opcode is immediately marked as "not verified"
and the experiment moves to the next opcode.

Aggregation is zkVM-wise.  The final report shows:
  - zkVM name
  - #chips / #opcodes in that VM
  - per-method: #verified-chips, #verified-ops, success-rate,
    mean ± std verification time (seconds)

Output
------
  * Conference-style ASCII table printed to stdout
  * CSV  written to  <output-dir>/comparison_results.csv
  * Per-trial raw YAML files kept in  <output-dir>/<vm>/<chip>/<method>/

Usage (run from repo root)
--------------------------
  python3 scripts/compare_experiments.py
  python3 scripts/compare_experiments.py --num-trial 5 --timeout-ms 60000
  python3 scripts/compare_experiments.py --methods bb,bb_blocking  # subset
  python3 scripts/compare_experiments.py --vms ziren,sp1           # subset of VMs
  python3 scripts/compare_experiments.py --skip-run                # just (re-)aggregate
  python3 scripts/compare_experiments.py --build                   # cargo build first
  python3 scripts/compare_experiments.py --workers 8               # run all tasks in parallel
  python3 scripts/compare_experiments.py --ablation                # ablation study mode
  python3 scripts/compare_experiments.py --ablation --methods bb,bb_no_heuristic  # subset
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import subprocess
import sys
import tempfile
import time
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

try:
    import yaml
except ImportError:
    print("ERROR: PyYAML not found.  Install with:  pip install pyyaml", file=sys.stderr)
    sys.exit(1)


# ── VM Registry ───────────────────────────────────────────────────────────────
# Complete chip→opcode mapping derived from each chip binary's get_opcode().
# For chips whose binary ignores --opcode-str (shiftleft, shiftright, sll, sr,
# pico/sr, ziren/shiftright, valida/lt32, valida/mul32, valida/memory) a single
# label is still listed so the binary is invoked exactly once with that label.

VM_REGISTRY: Dict[str, Dict] = {
    "ziren": {
        "dir": "examples/ziren",
        "chips": {
            "addsub":          ["ADD", "SUB"],
            "bitwise":         ["AND", "OR", "XOR"],
            "cloclz":          ["CLO", "CLZ"],
            "movcond":         ["MEQ", "MNE", "WSBH"],
            "mul":             ["MUL", "MULT", "MULTU"],
            "shiftleft":       ["SLL"],      # --opcode-str ignored
            "shiftright":      ["SRL"],      # --opcode-str ignored
            "divrem":          ["DIV", "DIVU", "MOD", "MODU"],
            "lt":              ["SLT", "SLTU"],
            "memoryreadwrite": ["LB", "LBU", "LH", "LHU", "LW",
                                "SB", "SH", "SW", "SC", "SWL", "SWR"],
            "jump":            ["Jump", "Jumpi", "JumpDirect"],
            "branch":          ["BEQ", "BNE", "BGEZ", "BGTZ", "BLEZ", "BLTZ"],
        },
    },
    "sp1": {
        "dir": "examples/sp1",
        "chips": {
            "addsub":          ["ADD", "SUB"],
            "bitwise":         ["AND", "OR", "XOR"],
            "mul":             ["MUL", "MULH", "MULHU", "MULHSU"],
            "divrem":          ["DIV", "DIVU", "REM", "REMU"],
            "lt":              ["SLT", "SLTU"],
            "shiftleft":       ["SLL"],      # --opcode-str ignored
            "sr":              ["SRL", "SRA"],
            "memoryreadwrite": ["LB", "LH", "LW", "LBU", "LHU",
                                "SB", "SH", "SW"],
            "jump":            ["JAL", "JALR"],
            "branch":          ["BEQ", "BGE", "BLT", "BNE"],
        },
    },
    "pico": {
        "dir": "examples/pico",
        "chips": {
            "addsub":          ["ADD", "SUB"],
            "bitwise":         ["AND", "OR", "XOR"],
            "mul":             ["MUL", "MULH", "MULHU", "MULHSU"],
            "divrem":          ["DIV", "DIVU", "REM", "REMU"],
            "lessthan":        ["SLT", "SLTU"],
            "sll":             ["SLL"],      # --opcode-str ignored
            "sr":              ["SRL"],      # --opcode-str ignored
            "memoryreadwrite": ["LB", "LBU", "LH", "LHU", "LW",
                                "SB", "SH", "SW"],
        },
    },
    "valida": {
        "dir": "examples/valida",
        "chips": {
            "add32":    ["ADD"],
            "sub32":    ["SUB"],
            "mul32":    ["MUL"],    # --opcode-str ignored
            "div32":    ["DIV", "SDIV"],
            "bitwise32":["AND", "OR", "XOR"],
            "com32":    ["EQ", "NE"],
            "lt32":     ["LT"],     # --opcode-str ignored
        },
    },
}

STANDARD_METHODS = ["bb", "bb_blocking", "z3"]
ABLATION_METHODS = ["bb_no_heuristic", "bb_no_simplify", "bb_no_refinement"]
ALL_METHODS      = STANDARD_METHODS + ABLATION_METHODS

# Human-readable labels for table headers
METHOD_LABELS: Dict[str, str] = {
    "bb":               "Branch-and-Bound (bb)",
    "bb_blocking":      "B&B + Blocking Closure",
    "z3":               "SMT Solver (z3)",
    "bb_no_heuristic":  "bb (no heuristic)",
    "bb_no_simplify":   "bb (no simplify)",
    "bb_no_refinement": "bb (no refinement)",
}

# Extra CLI flags passed to the binary for each method.
# The binary always receives --method bb (or z3); the dict captures only
# the ablation / variant flags that differ from plain bb.
METHOD_EXTRA_ARGS: Dict[str, List[str]] = {
    "bb":               [],
    "bb_blocking":      ["--blocking-closure"],
    "z3":               [],
    "bb_no_heuristic":  ["--no-heuristic"],
    "bb_no_simplify":   ["--no-simplify"],
    "bb_no_refinement": ["--no-refinement"],
}

# Which --method value to pass to the binary for each logical method
METHOD_BINARY_METHOD: Dict[str, str] = {
    "bb":               "bb",
    "bb_blocking":      "bb",
    "z3":               "z3",
    "bb_no_heuristic":  "bb",
    "bb_no_simplify":   "bb",
    "bb_no_refinement": "bb",
}


# ── Data structures ───────────────────────────────────────────────────────────

@dataclass
class OpcodeResult:
    vm:        str
    chip:      str
    opcode:    str
    method:    str
    verified:  bool          # True iff ALL completed trials succeeded
    times_s:   List[float]   # exe_time_mean of each successful trial
    n_run:     int           # trials attempted before early-stop
    n_success: int           # = len(times_s)
    skipped:   bool = False  # True if binary not found / not built


@dataclass
class MethodStats:
    n_verified_chips:  int   = 0
    n_verified_ops:    int   = 0
    success_rate:      float = 0.0   # n_verified_ops / n_total_ops * 100
    mean_time_s:       float = float("nan")
    std_time_s:        float = float("nan")
    all_times:         List[float] = field(default_factory=list)


@dataclass
class VMRow:
    vm:       str
    n_chips:  int
    n_ops:    int
    stats:    Dict[str, MethodStats] = field(default_factory=dict)


# ── Config helpers ────────────────────────────────────────────────────────────

def _write_temp_config(
    timeout_ms: int,
    seed: int,
    num_workers: int = 12,
    max_expansions: int = 30_000_000,
) -> str:
    cfg = {
        "time_out_ms":      timeout_ms,
        "num_workers":      num_workers,
        "max_expansions":   max_expansions,
        "seed":             seed,
    }
    fd, path = tempfile.mkstemp(suffix=".json", prefix="zebra_cfg_")
    with os.fdopen(fd, "w") as fh:
        json.dump(cfg, fh)
    return path


# ── Single-trial runner ───────────────────────────────────────────────────────

def _run_one_trial(
    vm_dir:     Path,
    chip:       str,
    opcode:     str,
    method:     str,
    config_path: str,
    timeout_ms: int,
    out_yaml:   str,
) -> Tuple[bool, float]:
    """
    Invoke the chip binary for one trial.
    Returns (success: bool, exe_time_s: float).
    """
    binary = vm_dir / "target" / "release" / "examples" / chip
    if not binary.exists():
        return False, 0.0

    # Safety wall-clock limit: 2× internal timeout + 30 s headroom
    wall_limit = timeout_ms / 1000.0 * 2.0 + 30.0

    binary_method = METHOD_BINARY_METHOD.get(method, method)
    extra_args    = METHOD_EXTRA_ARGS.get(method, [])

    cmd = [
        str(binary),
        "--config",           config_path,
        "--opcode-str",       opcode,
        "--method",           binary_method,
        "--num-trial",        "1",
        "--ouptput-path",     out_yaml,   # note: intentional typo matching quick.rs
        "--range-interval",   "0",
    ] + extra_args

    wall_start = time.monotonic()
    try:
        proc = subprocess.run(
            cmd,
            cwd=vm_dir,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=wall_limit,
        )
        wall_elapsed = time.monotonic() - wall_start

        if proc.returncode != 0:
            return False, wall_elapsed

        # Parse YAML written by the binary
        try:
            with open(out_yaml) as fh:
                data = yaml.safe_load(fh) or {}
            report = data.get("report", {})
            ratio  = float(report.get("success_ratio", 0.0))
            t_mean = float(report.get("exe_time_mean", wall_elapsed))
            return ratio >= 1.0, t_mean
        except Exception:
            return False, wall_elapsed

    except subprocess.TimeoutExpired:
        return False, wall_limit
    except FileNotFoundError:
        return False, 0.0


# ── Opcode-level experiment ───────────────────────────────────────────────────

def run_opcode(
    vm:              str,
    vm_dir:          Path,
    chip:            str,
    opcode:          str,
    method:          str,
    n_trials:        int,
    timeout_ms:      int,
    base_seed:       int,
    out_root:        Path,
    verbose:         bool = True,
    cfg_num_workers: int  = 12,
    tolerance:       int  = 0,
) -> OpcodeResult:
    """
    Run up to n_trials trials for one (chip, opcode, method).
    Early-stops after (tolerance + 1) failures.
    tolerance=0 (default / z3): stop on the first failure.
    tolerance=1 (bb methods):   allow one failure; stop on the second.
    Verified iff total failures <= tolerance.
    cfg_num_workers controls the num_workers written into the temp config
    (set to 1 when running many experiments in parallel to avoid CPU overload).
    """
    binary = vm_dir / "target" / "release" / "examples" / chip
    if not binary.exists():
        if verbose:
            _println(f"    [SKIP] binary not found: {binary}", flush=True)
        return OpcodeResult(vm, chip, opcode, method,
                            verified=False, times_s=[], n_run=0, n_success=0,
                            skipped=True)

    # Directory for saved per-trial YAMLs
    trial_dir = out_root / vm / chip / method
    trial_dir.mkdir(parents=True, exist_ok=True)

    times:     List[float] = []
    n_failures: int        = 0

    for i in range(n_trials):
        seed        = base_seed + i
        cfg_path    = _write_temp_config(timeout_ms, seed, num_workers=cfg_num_workers)
        yaml_out    = str(trial_dir / f"{opcode}_trial{i+1}.yaml")

        try:
            ok, t = _run_one_trial(vm_dir, chip, opcode, method,
                                   cfg_path, timeout_ms, yaml_out)
        finally:
            try:
                os.unlink(cfg_path)
            except OSError:
                pass

        if not ok:
            n_failures += 1

        if verbose:
            status_str = "ok" if ok else f"FAIL [{n_failures}/{tolerance + 1}]"
            _println(f"    trial {i+1}/{n_trials}  {status_str}  ({t:.3f}s)", flush=True)

        if not ok:
            if n_failures > tolerance:
                # Early stop
                return OpcodeResult(vm, chip, opcode, method,
                                    verified=False,
                                    times_s=times,
                                    n_run=i + 1,
                                    n_success=len(times))
        else:
            times.append(t)

    return OpcodeResult(vm, chip, opcode, method,
                        verified=n_failures <= tolerance,
                        times_s=times,
                        n_run=n_trials,
                        n_success=len(times))


# ── Full experiment loop ──────────────────────────────────────────────────────

_print_lock = threading.Lock()

# ESC[2K clears the entire current line; \r moves cursor to column 0.
# This is needed because the binaries use crossterm/ratatui which opens
# /dev/tty directly and leaves the terminal cursor at an arbitrary column.
_CLR = "\033[2K\r"


def _println(msg: str = "", **kwargs) -> None:
    """Print with a preceding line-clear so cursor pollution doesn't indent output."""
    print(_CLR + msg, **kwargs)


def _print_verdict(r: OpcodeResult) -> None:
    if r.skipped:
        verdict = "SKIPPED (binary not found)"
    elif r.verified:
        mean_t = sum(r.times_s) / len(r.times_s)
        std_t  = _std(r.times_s)
        verdict = f"VERIFIED  mean={mean_t:.3f}s  std={std_t:.3f}s"
    else:
        verdict = f"NOT verified  ({r.n_success}/{r.n_run} trials ok)"
    with _print_lock:
        _println(f"  --> [{r.vm}/{r.chip}/{r.opcode}] {r.method}  {verdict}", flush=True)


def run_experiments(
    vms:            List[str],
    methods:        List[str],
    n_trials:       int,
    timeout_ms:     int,
    base_seed:      int,
    out_root:       Path,
    repo_root:      Path,
    verbose:        bool  = True,
    workers:        int   = 1,
    timeout_scale:  float = 1.0,
    tolerance:      int   = 0,
) -> List[OpcodeResult]:
    """
    Run all (vm, chip, opcode, method) combinations.

    workers=1       — fully sequential (original behaviour).
    workers>1       — all tasks dispatched to a ThreadPoolExecutor; each task
                      uses cfg_num_workers=1 and a scaled-up timeout to account
                      for CPU sharing.
    timeout_scale   — multiply timeout_ms by this factor for each experiment.
                      When workers>1 the effective CPU share per task is ~1/workers,
                      so the same amount of work takes proportionally longer on the
                      wall clock.  Defaults to workers when workers>1 (auto-scale).
    tolerance       — allowed failures before early-stop; applied only to bb-based
                      methods (z3 always uses tolerance=0).
    """
    results: List[OpcodeResult] = []

    # When running in parallel each task gets ~1/workers of the CPU, so its
    # wall-clock time is ~workers× longer.  Scale the timeout accordingly so
    # experiments don't time out just because of contention.
    effective_timeout_ms = int(timeout_ms * timeout_scale)

    # Build a flat task list: (vm, vm_dir, chip, opcode, method)
    all_tasks = [
        (vm, repo_root / VM_REGISTRY[vm]["dir"], chip, opcode, method)
        for vm in vms
        for chip, opcodes in VM_REGISTRY[vm]["chips"].items()
        for opcode in opcodes
        for method in methods
    ]

    if workers == 1:
        # ── Sequential (original behaviour) ──────────────────────────────────
        cur_vm = None
        for vm, vm_dir, chip, opcode, method in all_tasks:
            if vm != cur_vm:
                chips = VM_REGISTRY[vm]["chips"]
                _println(flush=True)
                _println(f"{'='*60}", flush=True)
                _println(f"  zkVM: {vm}  ({len(chips)} chips)", flush=True)
                _println(f"{'='*60}", flush=True)
                cur_vm = vm
            _println(f"  [{vm}/{chip}/{opcode}] {method}", flush=True)
            r = run_opcode(
                vm=vm, vm_dir=vm_dir, chip=chip, opcode=opcode,
                method=method, n_trials=n_trials, timeout_ms=effective_timeout_ms,
                base_seed=base_seed, out_root=out_root, verbose=verbose,
                tolerance=tolerance if METHOD_BINARY_METHOD.get(method) == "bb" else 0,
            )
            results.append(r)
            _print_verdict(r)
    else:
        # ── Parallel ─────────────────────────────────────────────────────────
        # Each experiment uses cfg_num_workers=1 to avoid CPU overload when
        # many processes run simultaneously.
        with _print_lock:
            _println(f"Running {len(all_tasks)} tasks with {workers} parallel workers "
                     f"(cfg_num_workers=1, effective timeout={effective_timeout_ms}ms) ...",
                     flush=True)

        def _task(args):
            vm, vm_dir, chip, opcode, method = args
            return run_opcode(
                vm=vm, vm_dir=vm_dir, chip=chip, opcode=opcode,
                method=method, n_trials=n_trials, timeout_ms=effective_timeout_ms,
                base_seed=base_seed, out_root=out_root,
                verbose=False,      # suppress per-trial noise; verdicts printed below
                cfg_num_workers=1,  # each parallel task gets a single internal worker
                tolerance=tolerance if METHOD_BINARY_METHOD.get(method) == "bb" else 0,
            )

        with ThreadPoolExecutor(max_workers=workers) as pool:
            futures = {pool.submit(_task, t): t for t in all_tasks}
            for fut in as_completed(futures):
                r = fut.result()
                results.append(r)
                _print_verdict(r)

    return results


# ── Aggregation ───────────────────────────────────────────────────────────────

def _std(xs: List[float]) -> float:
    if len(xs) < 2:
        return 0.0
    m = sum(xs) / len(xs)
    return math.sqrt(sum((x - m) ** 2 for x in xs) / (len(xs) - 1))


def aggregate(
    results: List[OpcodeResult],
    vms: List[str],
    methods: List[str],
) -> List[VMRow]:
    rows: List[VMRow] = []

    for vm in vms:
        reg   = VM_REGISTRY[vm]
        chips = reg["chips"]
        n_chips = len(chips)
        n_ops   = sum(len(v) for v in chips.values())

        vm_results = [r for r in results if r.vm == vm]
        stats_map: Dict[str, MethodStats] = {}

        for method in methods:
            m_res = [r for r in vm_results if r.method == method]

            # Verified opcodes
            v_ops  = [r for r in m_res if r.verified]
            n_vops = len(v_ops)

            # Verified chips: a chip is verified iff ALL its opcodes are verified
            v_chip_set = set()
            for chip, opcodes in chips.items():
                chip_res = [r for r in m_res
                            if r.chip == chip and not r.skipped]
                if chip_res and all(r.verified for r in chip_res):
                    v_chip_set.add(chip)

            # Timing: pool all individual trial times from verified opcodes
            all_times: List[float] = []
            for r in v_ops:
                all_times.extend(r.times_s)

            mean_t = sum(all_times) / len(all_times) if all_times else float("nan")
            std_t  = _std(all_times) if len(all_times) >= 2 else 0.0
            succ   = n_vops / n_ops * 100.0 if n_ops else 0.0

            stats_map[method] = MethodStats(
                n_verified_chips = len(v_chip_set),
                n_verified_ops   = n_vops,
                success_rate     = succ,
                mean_time_s      = mean_t,
                std_time_s       = std_t,
                all_times        = all_times,
            )

        rows.append(VMRow(vm=vm, n_chips=n_chips, n_ops=n_ops, stats=stats_map))

    return rows


# ── Totals row ────────────────────────────────────────────────────────────────

def compute_totals(rows: List[VMRow], methods: List[str]) -> VMRow:
    total_chips = sum(r.n_chips for r in rows)
    total_ops   = sum(r.n_ops   for r in rows)
    total_stats: Dict[str, MethodStats] = {}

    for method in methods:
        all_times: List[float] = []
        n_vc = n_vo = 0
        for row in rows:
            s = row.stats.get(method, MethodStats())
            n_vc      += s.n_verified_chips
            n_vo      += s.n_verified_ops
            all_times.extend(s.all_times)
        mean_t = sum(all_times) / len(all_times) if all_times else float("nan")
        std_t  = _std(all_times) if len(all_times) >= 2 else 0.0
        succ   = n_vo / total_ops * 100.0 if total_ops else 0.0
        total_stats[method] = MethodStats(
            n_verified_chips = n_vc,
            n_verified_ops   = n_vo,
            success_rate     = succ,
            mean_time_s      = mean_t,
            std_time_s       = std_t,
            all_times        = all_times,
        )

    return VMRow(vm="TOTAL", n_chips=total_chips, n_ops=total_ops, stats=total_stats)


# ── Table printing ────────────────────────────────────────────────────────────

def _fmt_time(mean: float, std: float) -> str:
    if math.isnan(mean):
        return "     N/A     "
    return f"{mean:6.3f} ± {std:5.3f}"


def _fmt_pct(p: float) -> str:
    return f"{p:5.1f}%"


def print_table(rows: List[VMRow], methods: List[str]) -> None:
    """Print a conference-style ASCII comparison table."""

    totals = compute_totals(rows, methods)
    all_rows = rows + [totals]

    # Column widths
    w_vm    = max(len(r.vm) for r in all_rows) + 2
    w_vm    = max(w_vm, 8)

    # Per-method block header + data columns
    # Block:  V-Chips | V-Ops | Succ%  | Time (mean ± std)
    #           4     |  4    |  7     |    14
    blk_w   = 4 + 1 + 4 + 1 + 7 + 1 + 14   # = 32
    blk_w   = max(blk_w, 32)

    def sep(c="─", mid="┼", left="├", right="┤") -> str:
        parts = [c * (w_vm + 2), c * 6, c * 5]
        for _ in methods:
            parts.append(c * (blk_w + 2))
        return left + mid.join(parts) + right

    def row_line(vm, chips, ops, stats_map: Dict[str, MethodStats]) -> str:
        cells = [f" {vm:<{w_vm}} ", f" {chips:>4} ", f" {ops:>3} "]
        for method in methods:
            s = stats_map.get(method, MethodStats())
            vc  = f"{s.n_verified_chips:>4}"
            vo  = f"{s.n_verified_ops:>4}"
            pct = _fmt_pct(s.success_rate)
            t   = _fmt_time(s.mean_time_s, s.std_time_s)
            cells.append(f"  {vc} {vo}  {pct}  {t}  ")
        return "│" + "│".join(cells) + "│"

    # ── header ───────────────────────────────────────────────────────────────
    methods_str = " vs ".join(METHOD_LABELS.get(m, m) for m in methods)
    title = f"ZEBRA Verification — Cross-zkVM Comparison  (single-point, {methods_str})"
    outer_w = w_vm + 2 + 1 + 6 + 1 + 5 + 1 + len(methods) * (blk_w + 3)
    outer_w = max(outer_w, len(title) + 4)

    _println()
    _println("┌" + "─" * (outer_w) + "┐")
    _println("│" + f" {title} ".center(outer_w) + "│")
    _println("├" + "─" * (outer_w) + "┤")

    # Column group header
    meth_headers = "│".join(
        f"  {METHOD_LABELS.get(m, m):^{blk_w}}  "
        for m in methods
    )
    base_hdr = f" {'zkVM':<{w_vm}} │ {'#Chips':>4}  │ {'#Ops':>3}  "
    _println("│" + base_hdr + "│" + meth_headers + "│")

    # Sub-header
    def method_subhdr(m: str) -> str:
        return f"  {'V-Chips':>6}  {'V-Ops':>5}  {'Succ%':>6}  {'Time (s) mean ± std':^14}  "

    sub_base = f" {'':>{w_vm}} │ {'':>4}   │ {'':>3}   "
    sub_meths = "│".join(method_subhdr(m) for m in methods)
    _println("│" + sub_base + "│" + sub_meths + "│")

    _println(sep("─", "┼", "├", "┤"))

    # Data rows
    for r in rows:
        _println(row_line(r.vm, r.n_chips, r.n_ops, r.stats))

    # Totals
    _println(sep("═", "╪", "╞", "╡"))
    _println(row_line(totals.vm, totals.n_chips, totals.n_ops, totals.stats))
    _println("└" + "─" * outer_w + "┘")
    _println()


# ── Per-opcode detail table ───────────────────────────────────────────────────

def print_detail_table(results: List[OpcodeResult], methods: List[str]) -> None:
    """Print a per-opcode breakdown table."""
    # Widen method column to fit longest method name (e.g. "bb_blocking" = 11)
    w_method = max(len(m) for m in methods) if methods else 6
    w_method = max(w_method, 6)
    sep_m = "─" * (w_method + 2)
    hdr_m = f" {'Method':<{w_method}} "

    _println()
    _println(f"┌──────────┬────────────┬──────────┬{sep_m}┬────────────────────────────┐")
    _println(f"│          Per-Opcode Verification Detail{' ' * (w_method + 35)}│")
    _println(f"├──────────┬────────────┬──────────┬{sep_m}┬────────────────────────────┤")
    _println(f"│ zkVM     │ Chip       │ Opcode   │{hdr_m}│ Verdict  / Time (s)         │")
    _println(f"├──────────┼────────────┼──────────┼{sep_m}┼────────────────────────────┤")

    last_vm_chip = ("", "")
    for r in sorted(results, key=lambda x: (x.vm, x.chip, x.opcode, x.method)):
        vm_cell   = r.vm   if (r.vm, r.chip) != last_vm_chip else ""
        chip_cell = r.chip if (r.vm, r.chip) != last_vm_chip else ""
        last_vm_chip = (r.vm, r.chip)

        if r.skipped:
            verdict = "SKIPPED (no binary)"
        elif r.verified:
            mean_t = sum(r.times_s) / len(r.times_s)
            std_t  = _std(r.times_s)
            verdict = f"✓ VERIFIED  {mean_t:.3f} ± {std_t:.3f} s"
        else:
            verdict = f"✗ FAILED    ({r.n_success}/{r.n_run} ok)"

        _println(
            f"│ {vm_cell:<8} │ {chip_cell:<10} │ {r.opcode:<8} │ {r.method:<{w_method}} │ {verdict:<26} │"
        )

    _println(f"└──────────┴────────────┴──────────┴{sep_m}┴────────────────────────────┘")
    _println()


# ── CSV export ────────────────────────────────────────────────────────────────

def write_csv(
    rows:    List[VMRow],
    results: List[OpcodeResult],
    methods: List[str],
    csv_path: Path,
) -> None:
    totals = compute_totals(rows, methods)

    # ── Summary CSV (per-VM) ──────────────────────────────────────────────────
    summary_path = csv_path.parent / (csv_path.stem + "_summary.csv")
    summary_fields = ["zkvm", "num_chips", "num_opcodes"]
    for m in methods:
        summary_fields += [
            f"{m}_verified_chips",
            f"{m}_verified_ops",
            f"{m}_success_rate_pct",
            f"{m}_mean_time_s",
            f"{m}_std_time_s",
        ]

    with open(summary_path, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=summary_fields)
        w.writeheader()
        for row in rows + [totals]:
            record: Dict = {"zkvm": row.vm, "num_chips": row.n_chips, "num_opcodes": row.n_ops}
            for m in methods:
                s = row.stats.get(m, MethodStats())
                record[f"{m}_verified_chips"]    = s.n_verified_chips
                record[f"{m}_verified_ops"]      = s.n_verified_ops
                record[f"{m}_success_rate_pct"]  = round(s.success_rate, 2)
                record[f"{m}_mean_time_s"]       = (
                    round(s.mean_time_s, 6) if not math.isnan(s.mean_time_s) else ""
                )
                record[f"{m}_std_time_s"]        = (
                    round(s.std_time_s, 6) if not math.isnan(s.std_time_s) else ""
                )
            w.writerow(record)

    # ── Detail CSV (per-opcode) ───────────────────────────────────────────────
    detail_path = csv_path.parent / (csv_path.stem + "_detail.csv")
    detail_fields = [
        "zkvm", "chip", "opcode", "method",
        "verified", "n_trials_run", "n_trials_success",
        "mean_time_s", "std_time_s", "skipped",
    ]
    with open(detail_path, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=detail_fields)
        w.writeheader()
        for r in sorted(results, key=lambda x: (x.vm, x.chip, x.opcode, x.method)):
            mean_t = sum(r.times_s) / len(r.times_s) if r.times_s else ""
            std_t  = _std(r.times_s) if len(r.times_s) >= 2 else (0.0 if r.times_s else "")
            w.writerow({
                "zkvm":             r.vm,
                "chip":             r.chip,
                "opcode":           r.opcode,
                "method":           r.method,
                "verified":         r.verified,
                "n_trials_run":     r.n_run,
                "n_trials_success": r.n_success,
                "mean_time_s":      round(mean_t, 6) if isinstance(mean_t, float) else "",
                "std_time_s":       round(std_t,  6) if isinstance(std_t,  float) else "",
                "skipped":          r.skipped,
            })

    _println(f"CSV summary  saved to:  {summary_path}")
    _println(f"CSV detail   saved to:  {detail_path}")


# ── Load saved results (--skip-run) ──────────────────────────────────────────

def load_saved_results(
    out_root: Path,
    vms:      List[str],
    methods:  List[str],
) -> List[OpcodeResult]:
    """Re-read per-trial YAML files written by a previous run."""
    results: List[OpcodeResult] = []

    for vm in vms:
        reg   = VM_REGISTRY[vm]
        chips = reg["chips"]
        for chip, opcodes in chips.items():
            for opcode in opcodes:
                for method in methods:
                    trial_dir = out_root / vm / chip / method
                    times: List[float] = []
                    n_run = 0
                    n_ok  = 0

                    # Gather trial files in order
                    i = 1
                    while True:
                        yf = trial_dir / f"{opcode}_trial{i}.yaml"
                        if not yf.exists():
                            break
                        n_run += 1
                        try:
                            with open(yf) as fh:
                                data = yaml.safe_load(fh) or {}
                            report = data.get("report", {})
                            ratio  = float(report.get("success_ratio", 0.0))
                            t_mean = float(report.get("exe_time_mean", 0.0))
                            if ratio >= 1.0:
                                times.append(t_mean)
                                n_ok += 1
                            else:
                                # Early-stop means no more files should exist
                                break
                        except Exception:
                            break
                        i += 1

                    if n_run == 0:
                        continue  # no data for this combo; skip silently

                    # How many trials were expected?  If we have n_run files and
                    # the last one was successful, we ran all N without early-stop.
                    verified = (n_ok == n_run) and n_run > 0

                    results.append(OpcodeResult(
                        vm=vm, chip=chip, opcode=opcode, method=method,
                        verified=verified,
                        times_s=times,
                        n_run=n_run,
                        n_success=n_ok,
                    ))

    return results


# ── Build helper ──────────────────────────────────────────────────────────────

def build_vm(vm: str, repo_root: Path) -> bool:
    vm_dir = repo_root / VM_REGISTRY[vm]["dir"]
    print(f"  cargo build --release  [{vm}] ... ", end="", flush=True)
    proc = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=vm_dir,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if proc.returncode == 0:
        print("ok")
        return True
    else:
        print("FAILED")
        print(proc.stderr.decode(errors="replace"), file=sys.stderr)
        return False


# ── Main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="ZEBRA cross-zkVM comparison experiment",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--num-trial",   type=int,   default=5,
                        help="Trials per opcode (default: 5)")
    parser.add_argument("--timeout-ms",  type=int,   default=60_000,
                        help="Per-trial timeout in milliseconds (default: 60000)")
    parser.add_argument("--methods",     type=str,   default=None,
                        help="Comma-separated list of methods.  Defaults to standard methods "
                             "(bb,bb_blocking,z3) or ablation methods when --ablation is set.")
    parser.add_argument("--vms",         type=str,   default=",".join(VM_REGISTRY),
                        help=f"Comma-separated VMs (default: all)")
    parser.add_argument("--base-seed",   type=int,   default=41,
                        help="Base random seed; trial i uses seed+i (default: 41)")
    parser.add_argument("--output-dir",  type=str,   default=None,
                        help="Directory for per-trial YAML results "
                             "(default: experiments/comparison or experiments/ablation)")
    parser.add_argument("--csv-out",     type=str,   default=None,
                        help="Path for CSV outputs (default: <output-dir>/results.csv)")
    parser.add_argument("--skip-run",    action="store_true",
                        help="Skip running experiments; aggregate from existing --output-dir")
    parser.add_argument("--build",       action="store_true",
                        help="Run  cargo build --release  for each VM before experiments")
    parser.add_argument("--workers",        type=int,   default=1,
                        help="Parallel workers for all methods (default: 1 = sequential). "
                             "When >1, each experiment uses cfg_num_workers=1 internally.")
    parser.add_argument("--timeout-scale",  type=float, default=None,
                        help="Multiply --timeout-ms by this factor for each experiment. "
                             "Auto-computed as max(1, workers/cpu_count) when --workers>1 "
                             "(scales only when workers actually exceeds available cores).")
    parser.add_argument("--no-detail",   action="store_true",
                        help="Suppress per-opcode detail table")
    parser.add_argument("--quiet",       action="store_true",
                        help="Suppress per-trial progress output")
    parser.add_argument("--tolerance",   type=int, default=0,
                        help="Allowed failures before early-stop for bb-based methods "
                             "(default: 0).  z3 always uses tolerance=0.")
    parser.add_argument("--ablation",    action="store_true",
                        help="Run ablation study mode: bb vs bb_no_heuristic vs "
                             "bb_no_simplify vs bb_no_refinement.  "
                             "Changes the default --methods and --output-dir.")
    args = parser.parse_args()

    # ── Resolve mode-dependent defaults ──────────────────────────────────────
    if args.ablation:
        default_methods = "bb," + ",".join(ABLATION_METHODS)
        default_out_dir = "experiments/ablation"
    else:
        default_methods = ",".join(STANDARD_METHODS)
        default_out_dir = "experiments/comparison"

    methods_str = args.methods if args.methods is not None else default_methods
    methods     = [m.strip() for m in methods_str.split(",") if m.strip()]
    vms         = [v.strip() for v in args.vms.split(",")    if v.strip()]
    output_dir  = args.output_dir if args.output_dir is not None else default_out_dir

    # Auto-scale timeout for parallel runs.
    # When N workers share C CPUs the contention factor is max(1, N/C), NOT N.
    # On a 128-core server with --workers 64 there is no contention (scale=1);
    # on a 4-core laptop with --workers 64 the scale is 16×.
    # The user can always override with --timeout-scale.
    if args.timeout_scale is not None:
        timeout_scale = args.timeout_scale
    elif args.workers > 1:
        cpu_count = os.cpu_count() or 1
        timeout_scale = max(1.0, args.workers / cpu_count)
    else:
        timeout_scale = 1.0

    # Validate
    for vm in vms:
        if vm not in VM_REGISTRY:
            print(f"ERROR: unknown VM '{vm}'.  Known: {list(VM_REGISTRY)}", file=sys.stderr)
            sys.exit(1)
    for m in methods:
        if m not in ALL_METHODS:
            print(f"ERROR: unknown method '{m}'.  Known: {ALL_METHODS}", file=sys.stderr)
            sys.exit(1)

    repo_root = Path(__file__).resolve().parent.parent
    out_root  = (repo_root / output_dir).resolve()
    out_root.mkdir(parents=True, exist_ok=True)

    csv_path = Path(args.csv_out).resolve() if args.csv_out else out_root / "results.csv"
    csv_path.parent.mkdir(parents=True, exist_ok=True)

    # ── Optional build ────────────────────────────────────────────────────────
    if args.build:
        _println("\n=== Building VMs ===")
        for vm in vms:
            if not build_vm(vm, repo_root):
                print(f"Build failed for {vm}; aborting.", file=sys.stderr)
                sys.exit(1)

    # ── Run or load ───────────────────────────────────────────────────────────
    if args.skip_run:
        _println(f"\nLoading saved results from  {out_root} ...", flush=True)
        results = load_saved_results(out_root, vms, methods)
        if not results:
            print("ERROR: no saved results found.  Run without --skip-run first.",
                  file=sys.stderr)
            sys.exit(1)
    else:
        total_tasks = sum(
            len(VM_REGISTRY[vm]["chips"][chip]) * len(methods)
            for vm in vms
            for chip in VM_REGISTRY[vm]["chips"]
        )
        effective_ms = int(args.timeout_ms * timeout_scale)
        mode_label = "ablation study" if args.ablation else "standard comparison"
        _println(f"\nStarting experiment [{mode_label}]: {len(vms)} VMs, {len(methods)} methods, "
                 f"{args.num_trial} trials/opcode, timeout={args.timeout_ms}ms"
                 + (f" × {timeout_scale:.1f} = {effective_ms}ms (scaled)" if timeout_scale != 1.0 else "")
                 + f", tolerance={args.tolerance} (bb only)")
        _println(f"Total (chip×opcode×method) tasks: {total_tasks}")
        _println(f"Results directory: {out_root}")

        results = run_experiments(
            vms            = vms,
            methods        = methods,
            n_trials       = args.num_trial,
            timeout_ms     = args.timeout_ms,
            base_seed      = args.base_seed,
            out_root       = out_root,
            repo_root      = repo_root,
            verbose        = not args.quiet,
            workers        = args.workers,
            timeout_scale  = timeout_scale,
            tolerance      = args.tolerance,
        )

    # ── Aggregate ─────────────────────────────────────────────────────────────
    rows = aggregate(results, vms, methods)

    # ── Print tables ──────────────────────────────────────────────────────────
    print_table(rows, methods)

    if not args.no_detail:
        print_detail_table(results, methods)

    # ── Write CSV ─────────────────────────────────────────────────────────────
    write_csv(rows, results, methods, csv_path)


if __name__ == "__main__":
    main()
