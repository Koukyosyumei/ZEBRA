#!/usr/bin/env python3
"""
ZEBRA Cross-VM Experiment Plotter
==================================
Produces two figures saved under <repo>/figures/:
  - worker_sweep.png  : x = num_workers,   y = mean verification time
  - range_sweep.png   : x = range_interval, y = mean verification time

Each line/node = one (zkvm, chip, opcode) triple.
Error bars = ±1 std-dev (sqrt of exe_time_variance).
Colors are grouped by zkvm (same hue family) and shaded by chip.
Line styles distinguish multiple opcodes within the same chip.

Usage (from repo root):
  python3 scripts/plot_experiments.py [--base-dir PATH] [--out-dir PATH]
  python3 scripts/plot_experiments.py --only worker   # worker sweep only
  python3 scripts/plot_experiments.py --only range    # range  sweep only
"""

import argparse
import glob
import math
import os
from collections import defaultdict

import matplotlib.pyplot as plt
import matplotlib.cm as cm
import numpy as np
import yaml

plt.style.use("ggplot")

# ── VM / chip registry ────────────────────────────────────────────────────────
# cmap   : matplotlib colormap used to shade chips within this VM
# c_range: (lo, hi) – portion of the colormap to use (avoids near-white ends)
VMS = {
    "ziren": {
        "chips": ["addsub", "bitwise", "cloclz", "movcond", "mul", "shiftleft", "jump"],
        "cmap": "Reds",
        "c_range": (0.35, 0.90),
    },
    "sp1": {
        "chips": ["addsub", "bitwise", "mul", "jump", "shiftleft"],
        "cmap": "Blues",
        "c_range": (0.35, 0.90),
    },
    "pico": {
        "chips": ["addsub", "bitwise", "mul", "sll"],
        "cmap": "Greens",
        "c_range": (0.35, 0.90),
    },
    "valida": {
        "chips": ["add32", "sub32"],
        "cmap": "Purples",
        "c_range": (0.45, 0.80),
    },
}

# Line styles and markers cycle within a chip (one per opcode).
_LINESTYLES = ["-", "--", "-.", ":"]
_MARKERS    = ["o", "s", "^", "D", "v", "P", "*", "X"]


# ── data loading ──────────────────────────────────────────────────────────────

def load_sweep(vm_report_dir: str, sweep_type: str) -> dict:
    """
    Read all YAML result files under
      <vm_report_dir>/<sweep_type>/<chip>/<param_dir>/<OPCODE>.yaml

    Returns
      { (chip, opcode): { param_value: (mean, std) } }
    """
    base = os.path.join(vm_report_dir, sweep_type)
    if not os.path.isdir(base):
        return {}

    data: dict = defaultdict(dict)
    prefix = "workers_" if sweep_type == "worker_sweep" else "range_"

    for chip in sorted(os.listdir(base)):
        chip_path = os.path.join(base, chip)
        if not os.path.isdir(chip_path):
            continue

        for param_dir in sorted(os.listdir(chip_path)):
            if not param_dir.startswith(prefix):
                continue
            try:
                param_val = int(param_dir[len(prefix):])
            except ValueError:
                continue

            for ypath in sorted(glob.glob(os.path.join(chip_path, param_dir, "*.yaml"))):
                opcode = os.path.splitext(os.path.basename(ypath))[0]
                try:
                    with open(ypath) as fh:
                        content = yaml.safe_load(fh)
                except Exception:
                    continue
                if not content or "report" not in content:
                    continue
                rep  = content["report"]
                mean = rep.get("exe_time_mean")
                var  = rep.get("exe_time_variance", 0.0)
                if mean is None:
                    continue
                std = math.sqrt(max(var, 0.0))
                data[(chip, opcode)][param_val] = (mean, std)

    return data


# ── color helpers ─────────────────────────────────────────────────────────────

def chip_colors(chips: list[str], cmap_name: str, c_range: tuple) -> dict:
    """Map each chip to a RGBA colour sampled from the given colormap range."""
    cmap  = plt.get_cmap(cmap_name)
    lo, hi = c_range
    n     = max(len(chips), 1)
    locs  = np.linspace(lo, hi, n)
    return {chip: cmap(loc) for chip, loc in zip(chips, locs)}


# ── plotting ──────────────────────────────────────────────────────────────────

def plot_sweep(
    base_dir:   str,
    sweep_type: str,
    x_label:    str,
    out_path:   str,
) -> None:
    fig, ax = plt.subplots(figsize=(13, 7))

    legend_handles = []     # (handle, label) pairs, kept ordered for legend
    legend_labels  = []

    for vm_name, vm_cfg in VMS.items():
        vm_report_dir = os.path.join(base_dir, "examples", vm_name, "report")
        sweep_data    = load_sweep(vm_report_dir, sweep_type)
        if not sweep_data:
            continue

        colors = chip_colors(vm_cfg["chips"], vm_cfg["cmap"], vm_cfg["c_range"])

        # track opcode index per chip for linestyle / marker cycling
        chip_opcode_idx: dict[str, int] = defaultdict(int)

        # iterate in deterministic order: chip order defined in VMS, then alphabetic opcode
        for chip in vm_cfg["chips"]:
            relevant = sorted(
                [op for (c, op) in sweep_data if c == chip]
            )
            color = colors.get(chip)
            if color is None:
                continue

            for opcode in relevant:
                series = sweep_data.get((chip, opcode), {})
                if not series:
                    continue

                xs   = sorted(series.keys())
                ys   = [series[x][0] for x in xs]
                errs = [series[x][1] for x in xs]

                idx       = chip_opcode_idx[chip]
                linestyle = _LINESTYLES[idx % len(_LINESTYLES)]
                marker    = _MARKERS[idx % len(_MARKERS)]
                chip_opcode_idx[chip] += 1

                label = f"{vm_name} · {chip} · {opcode}"
                (line,) = ax.plot(
                    xs, ys,
                    color=color,
                    linestyle=linestyle,
                    marker=marker,
                    markersize=6,
                    linewidth=1.8,
                    label=label,
                )
                ax.errorbar(
                    xs, ys, yerr=errs,
                    fmt="none",
                    color=color,
                    capsize=4,
                    capthick=1.2,
                    elinewidth=1.0,
                    alpha=0.7,
                )
                legend_handles.append(line)
                legend_labels.append(label)

    # ── axes decoration ───────────────────────────────────────────────────────
    title = sweep_type.replace("_", " ").title()
    ax.set_title(f"ZEBRA – {title}", fontsize=14, fontweight="bold", pad=12)
    ax.set_xlabel(x_label, fontsize=11)
    ax.set_ylabel("Mean verification time (s)", fontsize=11)
    ax.tick_params(labelsize=9)

    # ── legend (outside, right side, grouped by VM via blank separators) ──────
    final_handles, final_labels = [], []
    current_vm = None
    for h, lbl in zip(legend_handles, legend_labels):
        vm = lbl.split(" · ")[0]
        if vm != current_vm:
            # blank separator line between VM groups
            if current_vm is not None:
                blank = plt.Line2D([], [], color="none")
                final_handles.append(blank)
                final_labels.append("")
            # VM section header (invisible line, bold label)
            header = plt.Line2D([], [], color="none")
            final_handles.append(header)
            final_labels.append(f"── {vm} ──")
            current_vm = vm
        final_handles.append(h)
        final_labels.append(lbl)

    legend = ax.legend(
        final_handles, final_labels,
        fontsize=7.5,
        loc="upper left",
        bbox_to_anchor=(1.01, 1.0),
        borderaxespad=0,
        framealpha=0.9,
        handlelength=2.5,
    )
    # bold the VM header lines
    for text in legend.get_texts():
        if text.get_text().startswith("──"):
            text.set_fontweight("bold")
            text.set_fontsize(8.5)

    fig.tight_layout(rect=[0, 0, 0.78, 1])   # leave room for legend

    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    print(f"Saved: {out_path}")
    plt.close(fig)


# ── main ──────────────────────────────────────────────────────────────────────

def find_repo_root(start: str) -> str:
    """Walk upward from *start* until we find an 'examples/' subdirectory."""
    path = os.path.abspath(start)
    for _ in range(6):
        if os.path.isdir(os.path.join(path, "examples")):
            return path
        path = os.path.dirname(path)
    return os.getcwd()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base-dir", default=None,
                        help="Root of the TwinVM repo (auto-detected if omitted)")
    parser.add_argument("--out-dir", default=None,
                        help="Output directory for figures (default: <base-dir>/figures)")
    parser.add_argument("--only", choices=["worker", "range"], default=None,
                        help="Run only one sweep type")
    args = parser.parse_args()

    base_dir = args.base_dir or find_repo_root(os.path.dirname(os.path.abspath(__file__)))
    out_dir  = args.out_dir  or os.path.join(base_dir, "figures")

    sweeps = [
        ("worker_sweep", "Number of workers",   "worker_sweep.png"),
        ("range_sweep",  "Range interval",       "range_sweep.png"),
    ]

    for sweep_type, x_label, fname in sweeps:
        if args.only == "worker" and sweep_type != "worker_sweep":
            continue
        if args.only == "range" and sweep_type != "range_sweep":
            continue
        plot_sweep(
            base_dir   = base_dir,
            sweep_type = sweep_type,
            x_label    = x_label,
            out_path   = os.path.join(out_dir, fname),
        )


if __name__ == "__main__":
    main()
