#!/usr/bin/env python3
"""
ZEBRA Cross-VM Experiment Plotter
==================================
Produces figures saved under <repo>/figures/:
  - worker_sweep_{granularity}.png
  - range_sweep_{granularity}.png

--granularity opcode  (default)
    One line per (chip, opcode).  Colors shaded by chip; line styles by opcode.

--granularity chip
    Opcodes within a chip are averaged into one line per chip.
    Error bar = pooled std across opcodes.

Usage (from repo root):
  python3 scripts/plot_experiments.py
  python3 scripts/plot_experiments.py --granularity chip
  python3 scripts/plot_experiments.py --only worker --granularity chip
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
    "sphinx": {
        "chips": ["addsub", "bitwise", "mul", "divrem", "lt", "shiftleft", "sr"],
        "cmap": "Oranges",
        "c_range": (0.35, 0.90),
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

    (chip, opcode) pairs where any parameter point has success_ratio < 1.0
    are excluded entirely.
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
                success_ratio = rep.get("success_ratio", 1.0)
                if mean is None:
                    continue
                # Skip individual points where verification did not fully succeed.
                # Opcodes with only partial data are still plotted up to the last
                # successful parameter value.
                if success_ratio < 1.0:
                    continue
                std = math.sqrt(max(var, 0.0))
                data[(chip, opcode)][param_val] = (mean, std)

    return data


# ── chip-level aggregation ────────────────────────────────────────────────────

def aggregate_to_chip(sweep_data: dict) -> dict:
    """
    Collapse  { (chip, opcode): {param: (mean, std)} }
    into      { chip:           {param: (mean, std)} }

    At each param value:
      aggregated mean = arithmetic mean of per-opcode means
      aggregated std  = sqrt( mean(std_i²) + var(mean_i) )
                        (pooled within-opcode noise + between-opcode spread)
    """
    # collect all param values and means per chip
    chip_series: dict[str, dict[int, list]] = defaultdict(lambda: defaultdict(list))
    for (chip, _opcode), series in sweep_data.items():
        for param, (mean, std) in series.items():
            chip_series[chip][param].append((mean, std))

    result: dict[str, dict] = {}
    for chip, param_map in chip_series.items():
        result[chip] = {}
        for param, entries in param_map.items():
            means = [m for m, _ in entries]
            stds  = [s for _, s in entries]
            agg_mean = sum(means) / len(means)
            # pooled std: sqrt( mean(var_i) + var(mean_i) )
            mean_var   = sum(s ** 2 for s in stds) / len(stds)
            between_var = sum((m - agg_mean) ** 2 for m in means) / len(means)
            agg_std = math.sqrt(mean_var + between_var)
            result[chip][param] = (agg_mean, agg_std)
    return result


# ── color helpers ─────────────────────────────────────────────────────────────

def chip_colors(chips: list[str], cmap_name: str, c_range: tuple) -> dict:
    """Map each chip to a RGBA colour sampled from the given colormap range."""
    cmap  = plt.get_cmap(cmap_name)
    lo, hi = c_range
    n     = max(len(chips), 1)
    locs  = np.linspace(lo, hi, n)
    return {chip: cmap(loc) for chip, loc in zip(chips, locs)}


# ── plotting ──────────────────────────────────────────────────────────────────

def _plot_vm_ax(
    ax,
    vm_name:     str,
    vm_cfg:      dict,
    sweep_data:  dict,
    granularity: str = "opcode",   # "chip" | "opcode"
    x_transform = None,            # optional callable: raw param → display value
    x_log:       bool = False,     # use log scale on x-axis
) -> None:
    """Draw lines for one VM onto *ax*, at chip or opcode granularity."""
    xfn = x_transform if x_transform is not None else (lambda v: v)
    colors = chip_colors(vm_cfg["chips"], vm_cfg["cmap"], vm_cfg["c_range"])

    if granularity == "chip":
        # ── one line per chip ──────────────────────────────────────────────
        chip_data = aggregate_to_chip(sweep_data)
        for i, chip in enumerate(vm_cfg["chips"]):
            series = chip_data.get(chip)
            if not series:
                continue
            color = colors.get(chip)
            xs       = [xfn(v) for v in sorted(series.keys())]
            ys       = [series[v][0] for v in sorted(series.keys())]
            errs_raw = [series[v][1] for v in sorted(series.keys())]
            errs_lo  = [min(e, y * 0.9999) for y, e in zip(ys, errs_raw)]
            errs     = [errs_lo, errs_raw]
            marker   = _MARKERS[i % len(_MARKERS)]
            ax.plot(xs, ys, color=color, linestyle="-", marker=marker,
                    markersize=6, linewidth=2.0, label=chip)
            """
            ax.errorbar(xs, ys, yerr=errs, fmt="none", color=color,
                        capsize=4, capthick=1.1, elinewidth=1.0, alpha=0.7)
            """

    else:
        # ── one line per (chip, opcode) ────────────────────────────────────
        chip_opcode_idx: dict[str, int] = defaultdict(int)
        for chip in vm_cfg["chips"]:
            relevant = sorted([op for (c, op) in sweep_data if c == chip])
            color = colors.get(chip)
            if color is None:
                continue
            for opcode in relevant:
                series = sweep_data.get((chip, opcode), {})
                if not series:
                    continue
                xs       = [xfn(v) for v in sorted(series.keys())]
                ys       = [series[v][0] for v in sorted(series.keys())]
                errs_raw = [series[v][1] for v in sorted(series.keys())]
                errs_lo  = [min(e, y * 0.9999) for y, e in zip(ys, errs_raw)]
                errs     = [errs_lo, errs_raw]
                idx       = chip_opcode_idx[chip]
                linestyle = _LINESTYLES[idx % len(_LINESTYLES)]
                marker    = _MARKERS[idx % len(_MARKERS)]
                chip_opcode_idx[chip] += 1
                ax.plot(xs, ys, color=color, linestyle=linestyle, marker=marker,
                        markersize=5, linewidth=1.6, label=f"{chip} · {opcode}")
                """
                ax.errorbar(xs, ys, yerr=errs, fmt="none", color=color,
                            capsize=3, capthick=1.0, elinewidth=0.9, alpha=0.7)
                """

    ax.set_title(vm_name, fontsize=11, fontweight="bold")
    ax.set_yscale("log")
    ax.yaxis.set_major_formatter(
        plt.matplotlib.ticker.LogFormatterSciNotation(labelOnlyBase=False)
    )
    if x_log:
        ax.set_xscale("log")
        ax.xaxis.set_major_formatter(plt.matplotlib.ticker.ScalarFormatter())
    ax.grid(True, which="both", axis="y", linestyle=":", linewidth=0.5, alpha=0.6)
    ax.tick_params(labelsize=11)
    ax.legend(fontsize=9, loc="best", bbox_to_anchor=(1.0, 1.0),
              framealpha=0.85, handlelength=2.0)

    if not sweep_data:
        ax.text(0.5, 0.5, "no data", transform=ax.transAxes,
                ha="center", va="center", color="gray", fontsize=9)


def plot_sweep(
    base_dir:    str,
    sweep_type:  str,
    x_label:     str,
    out_path:    str,
    granularity: str = "opcode",   # "chip" | "opcode"
) -> None:
    # For range sweeps, display the search-space volume (x+1)^2 instead of x.
    x_transform = (lambda v: (v + 1) ** 2) if sweep_type == "range_sweep" else None

    vm_names = list(VMS.keys())
    n = len(vm_names)

    fig, axes = plt.subplots(1, n, figsize=(5 * n, 5), sharey=False)
    for ax, vm_name in zip(axes, vm_names):
        vm_cfg        = VMS[vm_name]
        vm_report_dir = os.path.join(base_dir, "examples", vm_name, "report")
        sweep_data    = load_sweep(vm_report_dir, sweep_type)

        _plot_vm_ax(ax, vm_name, vm_cfg, sweep_data, granularity=granularity,
                    x_transform=x_transform,
                    x_log=(sweep_type == "range_sweep"))
        ax.set_xlabel(x_label, fontsize=13)

    axes[0].set_ylabel("Mean verification time (s)  [log]", fontsize=13)

    fig.tight_layout()
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    pdf_path = os.path.splitext(out_path)[0] + ".pdf"
    fig.savefig(pdf_path, format="pdf", bbox_inches="tight")
    print(f"Saved: {pdf_path}")
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
    parser.add_argument("--granularity", choices=["chip", "opcode"], default="opcode",
                        help="chip: one line per chip (opcodes averaged); "
                             "opcode: one line per opcode (default)")
    args = parser.parse_args()

    base_dir = args.base_dir or find_repo_root(os.path.dirname(os.path.abspath(__file__)))
    out_dir  = args.out_dir  or os.path.join(base_dir, "figures")
    gran     = args.granularity

    sweeps = [
        ("worker_sweep", "Number of workers",          f"worker_sweep_{gran}.pdf"),
        ("range_sweep",  "input volume",                 f"range_sweep_{gran}.pdf"),
    ]

    for sweep_type, x_label, fname in sweeps:
        if args.only == "worker" and sweep_type != "worker_sweep":
            continue
        if args.only == "range" and sweep_type != "range_sweep":
            continue
        plot_sweep(
            base_dir    = base_dir,
            sweep_type  = sweep_type,
            x_label     = x_label,
            out_path    = os.path.join(out_dir, fname),
            granularity = gran,
        )


if __name__ == "__main__":
    main()
