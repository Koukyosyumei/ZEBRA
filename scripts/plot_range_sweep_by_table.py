#!/usr/bin/env python3
"""
ZEBRA Range-Sweep Plot by Table Type
======================================
For each canonical table type (addsub, bitwise, mul, …), produces one subplot
showing verification time vs. verified input volume across all zkVMs that have
that table.  Each VM gets a solid line; a matching dashed line shows the
hypothetical brute-force cost (singleton_time × volume).

Also produces a companion gain table (LaTeX / Markdown / CSV / plain) where
  gain = volume × singleton_time / range_verification_time
averaged over all chips/opcodes per (zkVM, volume) cell.

Usage (from repo root):
  python3 scripts/plot_range_sweep_by_table.py
  python3 scripts/plot_range_sweep_by_table.py --format latex
  python3 scripts/plot_range_sweep_by_table.py --out-dir figures/
"""

import argparse
import glob
import math
import os
import sys
from collections import defaultdict

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np
import yaml

plt.style.use("ggplot")

# ── canonical table type → {vm: chip_name_in_report_dir} ─────────────────────
# Keys are the "table types" shown in paper.  Values map vm_name → the chip
# directory name used in  examples/{vm}/report/range_sweep/{chip}/  .
# A VM is omitted from a table type when it simply does not have that chip.
TABLE_TYPES: dict[str, dict[str, str]] = {
    "addsub": {
        "ziren":  "addsub",
        "sp1":    "addsub",
        "pico":   "addsub",
        "valida": "addsub",   # valida uses add32/sub32; aggregated below
        "sphinx": "addsub",
    },
    "bitwise": {
        "ziren":  "bitwise",
        "sp1":    "bitwise",
        "pico":   "bitwise",
        "sphinx": "bitwise",
    },
    "mul": {
        "ziren":  "mul",
        "sp1":    "mul",
        "pico":   "mul",
        "sphinx": "mul",
    },
    "divrem": {
        "sp1":    "divrem",
        "pico":   "divrem",
        "sphinx": "divrem",
    },
    "lt": {
        "sp1":    "lt",
        "pico":   "lessthan",
        "sphinx": "lt",
    },
    "shiftleft": {
        "ziren":  "shiftleft",
        "sp1":    "shiftleft",
        "pico":   "sll",
        "sphinx": "shiftleft",
    },
    "sr": {
        "sp1":    "sr",
        "pico":   "sr",
        "sphinx": "sr",
    },
    "jump": {
        "ziren": "jump",
        "sp1":   "jump",
    },
}

# valida uses split chips; map canonical "addsub" to both
VALIDA_ADDSUB_CHIPS = ["add32", "sub32"]

# Colour per zkVM (consistent with compare_experiments.py palette)
VM_COLORS = {
    "ziren":  "#d62728",   # red
    "sp1":    "#1f77b4",   # blue
    "pico":   "#2ca02c",   # green
    "valida": "#9467bd",   # purple
    "sphinx": "#ff7f0e",   # orange
}
VM_MARKERS = {
    "ziren":  "o",
    "sp1":    "s",
    "pico":   "^",
    "valida": "D",
    "sphinx": "P",
}
VM_ORDER = ["valida", "sphinx", "pico", "sp1", "ziren"]

# ── data loading ──────────────────────────────────────────────────────────────

def load_range_sweep_chip(vm_report_dir: str, chip: str) -> dict[str, dict[int, tuple[float, float]]]:
    """
    Load range_sweep data for one chip directory.
    Returns { opcode: { range_val: (mean_s, std_s) } }.
    Only entries with success_ratio == 1.0 are kept.
    """
    base = os.path.join(vm_report_dir, "range_sweep", chip)
    if not os.path.isdir(base):
        return {}

    data: dict[str, dict[int, tuple[float, float]]] = defaultdict(dict)
    for param_dir in sorted(os.listdir(base)):
        if not param_dir.startswith("range_"):
            continue
        try:
            range_val = int(param_dir[len("range_"):])
        except ValueError:
            continue
        for ypath in sorted(glob.glob(os.path.join(base, param_dir, "*.yaml"))):
            opcode = os.path.splitext(os.path.basename(ypath))[0]
            try:
                with open(ypath) as fh:
                    content = yaml.safe_load(fh)
            except Exception:
                continue
            if not content or "report" not in content:
                continue
            rep = content["report"]
            mean = rep.get("exe_time_mean")
            var  = rep.get("exe_time_variance", 0.0)
            if rep.get("success_ratio", 1.0) < 1.0 or mean is None:
                continue
            data[opcode][range_val] = (mean, math.sqrt(max(var, 0.0)))
    return data


def load_table_type_vm(
    base_dir: str,
    vm: str,
    canonical_chip: str,
    actual_chip: str,
) -> dict[str, dict[int, tuple[float, float]]]:
    """Load range_sweep data for one (vm, canonical_chip) pair."""
    vm_report_dir = os.path.join(base_dir, "examples", vm, "report")

    if vm == "valida" and canonical_chip == "addsub":
        # Merge add32 and sub32
        merged: dict[str, dict[int, tuple[float, float]]] = {}
        for chip_name in VALIDA_ADDSUB_CHIPS:
            for op, series in load_range_sweep_chip(vm_report_dir, chip_name).items():
                merged[op] = series
        return merged

    return load_range_sweep_chip(vm_report_dir, actual_chip)


def aggregate_opcodes(
    opcode_data: dict[str, dict[int, tuple[float, float]]]
) -> dict[int, tuple[float, float]]:
    """
    Collapse {opcode: {range_val: (mean, std)}} into {range_val: (agg_mean, agg_std)}.
    Only range values present in ALL opcodes are kept (inner join).
    """
    if not opcode_data:
        return {}
    opcodes = list(opcode_data.keys())
    # Intersection of range values across all opcodes
    common_ranges = set(opcode_data[opcodes[0]].keys())
    for op in opcodes[1:]:
        common_ranges &= set(opcode_data[op].keys())

    result: dict[int, tuple[float, float]] = {}
    for rv in sorted(common_ranges):
        means = [opcode_data[op][rv][0] for op in opcodes]
        stds  = [opcode_data[op][rv][1] for op in opcodes]
        agg_mean = sum(means) / len(means)
        mean_var   = sum(s ** 2 for s in stds) / len(stds)
        between_var = sum((m - agg_mean) ** 2 for m in means) / len(means)
        agg_std = math.sqrt(mean_var + between_var)
        result[rv] = (agg_mean, agg_std)
    return result


# ── plotting ──────────────────────────────────────────────────────────────────

def _draw_vm_ax(ax, vm: str, series: dict[int, tuple[float, float]]) -> None:
    """
    Draw one VM's actual line + brute-force reference onto *ax*.
    x = verified volume (r+1)^2, y = mean verification time (s).
    """
    color  = VM_COLORS.get(vm, "gray")
    marker = VM_MARKERS.get(vm, "o")

    xs      = sorted(series.keys())
    volumes = [(r + 1) ** 2 for r in xs]
    ys      = [series[r][0] for r in xs]

    ax.plot(volumes, ys,
            color=color, marker=marker, markersize=5,
            linewidth=2.0, linestyle="-", label="ZEBRA")

    singleton_time = series.get(0, (None, None))[0]
    if singleton_time is not None:
        bf_ys = [singleton_time * v for v in volumes]
        ax.plot(volumes, bf_ys,
                color=color, linestyle="--", linewidth=1.2,
                alpha=0.55, marker="", label="iterative point-wise verification")

    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("Verified input volume", fontsize=13)
    ax.set_ylabel("Mean verification time (s)", fontsize=13)
    ax.xaxis.set_major_formatter(ticker.ScalarFormatter())
    ax.yaxis.set_major_formatter(
        ticker.LogFormatterSciNotation(labelOnlyBase=False)
    )
    ax.tick_params(labelsize=12)
    ax.grid(True, which="both", axis="both",
            linestyle=":", linewidth=0.5, alpha=0.6)
    ax.legend(fontsize=11, loc="upper left", framealpha=0.85, handlelength=2.2)


def _draw_vm_ax_inverted(ax, vm: str, series: dict[int, tuple[float, float]]) -> None:
    """
    Draw one VM's actual line + brute-force reference onto *ax*.
    x = mean verification time (s), y = verified volume (r+1)^2.
    """
    color  = VM_COLORS.get(vm, "gray")
    marker = VM_MARKERS.get(vm, "o")

    xs      = sorted(series.keys())
    volumes = [(r + 1) ** 2 for r in xs]
    times   = [series[r][0] for r in xs]

    ax.plot(times, volumes,
            color=color, marker=marker, markersize=5,
            linewidth=2.0, linestyle="-", label="ZEBRA")

    singleton_time = series.get(0, (None, None))[0]
    if singleton_time is not None and singleton_time > 0:
        # brute-force: time = singleton_time × volume  →  volume = time / singleton_time
        bf_vols = [t / singleton_time for t in times]
        ax.plot(times, bf_vols,
                color=color, linestyle="--", linewidth=1.2,
                alpha=0.55, marker="", label="iterative point-wise verification")

    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("Mean verification time (s)", fontsize=13)
    ax.set_ylabel("Verified input volume", fontsize=13)
    ax.xaxis.set_major_formatter(
        ticker.LogFormatterSciNotation(labelOnlyBase=False)
    )
    ax.yaxis.set_major_formatter(ticker.ScalarFormatter())
    ax.tick_params(labelsize=12)
    ax.grid(True, which="both", axis="both",
            linestyle=":", linewidth=0.5, alpha=0.6)
    ax.legend(fontsize=11, loc="upper left", framealpha=0.85, handlelength=2.2)


def _save_fig(fig, out_dir: str, stem: str) -> None:
    for ext in ("pdf", "png"):
        path = os.path.join(out_dir, f"{stem}.{ext}")
        fig.savefig(path,
                    format=ext,
                    dpi=(150 if ext == "png" else None),
                    bbox_inches="tight")
        print(f"Saved: {path}")


def plot_by_table_type(base_dir: str, out_dir: str) -> None:
    """
    For each table type: one figure with one subfigure per zkVM.
    Each subfigure: x = volume, y = verification time (log-log).
    A second set of figures swaps the axes (x = time, y = volume).
    """
    os.makedirs(out_dir, exist_ok=True)

    any_data = False
    for ttype, vm_map in TABLE_TYPES.items():
        vm_series: dict[str, dict[int, tuple[float, float]]] = {}
        for vm, chip in vm_map.items():
            opdata = load_table_type_vm(base_dir, vm, ttype, chip)
            if not opdata:
                continue
            agg = aggregate_opcodes(opdata)
            if agg:
                vm_series[vm] = agg

        if not vm_series:
            continue

        any_data = True
        vms_present = [vm for vm in VM_ORDER if vm in vm_series]
        n = len(vms_present)

        # ── figure 1: volume (x) vs time (y) ─────────────────────────────────
        fig, axes = plt.subplots(1, n, figsize=(4 * n, 4), sharey=False)
        if n == 1:
            axes = [axes]
        for ax, vm in zip(axes, vms_present):
            _draw_vm_ax(ax, vm, vm_series[vm])
            ax.set_title(vm, fontsize=16, fontweight="bold")
        fig.tight_layout()
        _save_fig(fig, out_dir, f"range_sweep_{ttype}")
        plt.close(fig)

        # ── figure 2: time (x) vs volume (y) ─────────────────────────────────
        fig2, axes2 = plt.subplots(1, n, figsize=(4 * n, 4), sharey=False)
        if n == 1:
            axes2 = [axes2]
        for ax, vm in zip(axes2, vms_present):
            _draw_vm_ax_inverted(ax, vm, vm_series[vm])
            ax.set_title(vm, fontsize=16, fontweight="bold")
        fig2.tight_layout()
        _save_fig(fig2, out_dir, f"range_sweep_{ttype}_inverted")
        plt.close(fig2)

    if not any_data:
        print("No range_sweep data found.  Run the range-sweep experiments first.",
              file=sys.stderr)


# ── gain table ────────────────────────────────────────────────────────────────

def build_gain_table(
    base_dir: str,
    vms: list[str] | None = None,
) -> tuple[list[str], list[int], dict[str, dict[int, float]]]:
    """
    Returns (vm_names, sorted_volumes, gain_table)
    where gain_table[vm][volume] = mean gain across all table types & opcodes.

    gain = volume × singleton_time / verification_time   (per opcode per range)
    """
    if vms is None:
        vms = list(VM_COLORS.keys())

    all_volumes: set[int] = set()
    # gains_raw[vm][(range_val)] = list of per-opcode gains
    gains_raw: dict[str, dict[int, list[float]]] = {vm: defaultdict(list) for vm in vms}

    for ttype, vm_map in TABLE_TYPES.items():
        for vm, chip in vm_map.items():
            if vm not in vms:
                continue
            opdata = load_table_type_vm(base_dir, vm, ttype, chip)
            for opcode, series in opdata.items():
                singleton_time = series.get(0, (None, None))[0]
                if singleton_time is None or singleton_time == 0:
                    continue
                for rv, (mean, _) in series.items():
                    if rv == 0 or mean == 0:
                        continue
                    volume = (rv + 1) ** 2
                    gain = volume * singleton_time / mean
                    gains_raw[vm][rv].append(gain)
                    all_volumes.add(rv)

    volumes_sorted = sorted(all_volumes)
    gain_table: dict[str, dict[int, float]] = {}
    for vm in vms:
        gain_table[vm] = {}
        for rv in volumes_sorted:
            vals = gains_raw[vm].get(rv, [])
            if vals:
                gain_table[vm][(rv + 1) ** 2] = sum(vals) / len(vals)

    volume_labels = sorted({(rv + 1) ** 2 for rv in volumes_sorted})
    return vms, volume_labels, gain_table


# ── table formatters ──────────────────────────────────────────────────────────

def _gcell(table: dict, vm: str, vol: int, decimals: int = 1) -> str:
    v = table[vm].get(vol)
    return f"{v:.{decimals}f}×" if v is not None else "—"


def fmt_plain(vms, volumes, table) -> str:
    vol_w = max(len(str(v)) + 1 for v in volumes)
    vol_w = max(vol_w, 8)
    vm_w  = max(len(v) for v in vms)
    header = f"{'zkVM':<{vm_w}}" + "".join(f"  {v:>{vol_w}}" for v in volumes)
    sep    = "-" * len(header)
    rows   = [header, sep]
    for vm in vms:
        row = f"{vm:<{vm_w}}" + "".join(
            f"  {_gcell(table, vm, vol):>{vol_w}}" for vol in volumes
        )
        rows.append(row)
    return "\n".join(rows)


def fmt_csv(vms, volumes, table) -> str:
    lines = ["zkVM," + ",".join(str(v) for v in volumes)]
    for vm in vms:
        vals = ",".join(_gcell(table, vm, vol) for vol in volumes)
        lines.append(f"{vm},{vals}")
    return "\n".join(lines)


def fmt_markdown(vms, volumes, table) -> str:
    header = "| zkVM | " + " | ".join(str(v) for v in volumes) + " |"
    sep    = "|:-----|" + "|".join("---:" for _ in volumes) + "|"
    rows   = [header, sep]
    for vm in vms:
        vals = " | ".join(_gcell(table, vm, vol) for vol in volumes)
        rows.append(f"| {vm} | {vals} |")
    return "\n".join(rows)


def fmt_latex(vms, volumes, table) -> str:
    cols = "l" + "r" * len(volumes)
    vol_hdrs = " & ".join(f"vol={v}" for v in volumes)
    lines = [
        r"\begin{tabular}{" + cols + "}",
        r"\toprule",
        r"zkVM & " + vol_hdrs + r" \\",
        r"\midrule",
    ]
    for vm in vms:
        vals = " & ".join(_gcell(table, vm, vol) for vol in volumes)
        lines.append(f"{vm} & {vals} \\\\")
    lines += [r"\bottomrule", r"\end{tabular}"]
    return "\n".join(lines)


FORMATTERS = {
    "plain":    fmt_plain,
    "csv":      fmt_csv,
    "markdown": fmt_markdown,
    "latex":    fmt_latex,
}


# ── repo root detection ───────────────────────────────────────────────────────

def find_repo_root(start: str) -> str:
    path = os.path.abspath(start)
    for _ in range(6):
        if os.path.isdir(os.path.join(path, "examples")):
            return path
        path = os.path.dirname(path)
    return os.getcwd()


# ── main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--base-dir", default=None,
                        help="Root of the ZEBRA repo (auto-detected)")
    parser.add_argument("--out-dir", default=None,
                        help="Output directory for figures (default: <base>/figures)")
    parser.add_argument("--format", choices=list(FORMATTERS), default="plain",
                        help="Gain table output format (default: plain)")
    parser.add_argument("--table-out", default=None,
                        help="Write gain table to this file (default: stdout)")
    parser.add_argument("--no-plot", action="store_true",
                        help="Skip the figure; only produce the gain table")
    parser.add_argument("--no-table", action="store_true",
                        help="Skip the gain table; only produce the figure")
    args = parser.parse_args()

    base_dir = args.base_dir or find_repo_root(
        os.path.dirname(os.path.abspath(__file__))
    )
    out_dir = args.out_dir or os.path.join(base_dir, "figures")

    # ── plot ──────────────────────────────────────────────────────────────────
    if not args.no_plot:
        plot_by_table_type(base_dir, out_dir)

    # ── gain table ────────────────────────────────────────────────────────────
    if not args.no_table:
        vms, volumes, gain_table = build_gain_table(base_dir)

        if not volumes:
            print("No range_sweep data found for gain table.", file=sys.stderr)
        else:
            output = FORMATTERS[args.format](vms, volumes, gain_table)
            if args.table_out:
                os.makedirs(os.path.dirname(os.path.abspath(args.table_out)), exist_ok=True)
                with open(args.table_out, "w") as fh:
                    fh.write(output + "\n")
                print(f"Gain table saved: {args.table_out}")
            else:
                print("\n=== Speedup Gain Table  (avg gain = vol × t_singleton / t_range) ===")
                print(output)


if __name__ == "__main__":
    main()
