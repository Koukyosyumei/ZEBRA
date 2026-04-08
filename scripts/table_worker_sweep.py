#!/usr/bin/env python3
"""
ZEBRA Worker-Sweep Table Generator
====================================
Produces a table where:
  - Rows    : each zkVM
  - Columns : number of workers
  - Cells   : mean verification time (s), averaged over all chips/opcodes

Usage (from repo root):
  python3 scripts/table_worker_sweep.py
  python3 scripts/table_worker_sweep.py --base-dir /path/to/repo
  python3 scripts/table_worker_sweep.py --format csv
  python3 scripts/table_worker_sweep.py --format markdown
  python3 scripts/table_worker_sweep.py --format latex
"""

import argparse
import os
import sys

# Re-use helpers from plot_experiments.py (same directory)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from plot_experiments import VMS, load_sweep, find_repo_root


def build_table(base_dir: str) -> tuple[list[str], list[int], dict[str, dict[int, float]]]:
    """
    Returns
      vm_names  : ordered list of VM names
      workers   : sorted list of worker counts seen across all VMs
      table     : { vm_name: { n_workers: avg_mean_time } }
    """
    all_workers: set[int] = set()
    table: dict[str, dict[int, float]] = {}

    for vm_name in VMS:
        vm_report_dir = os.path.join(base_dir, "examples", vm_name, "report")
        sweep_data = load_sweep(vm_report_dir, "worker_sweep")

        # Aggregate all (chip, opcode) series into one mean per worker count
        worker_means: dict[int, list[float]] = {}
        for series in sweep_data.values():
            for n_workers, (mean, _std) in series.items():
                worker_means.setdefault(n_workers, []).append(mean)

        table[vm_name] = {
            n: sum(vals) / len(vals) for n, vals in worker_means.items()
        }
        all_workers.update(worker_means.keys())

    workers = sorted(all_workers)
    return list(VMS.keys()), workers, table


# ── formatters ────────────────────────────────────────────────────────────────

def _cell(table: dict, vm: str, w: int) -> str:
    v = table[vm].get(w)
    return f"{v:.4f}" if v is not None else "—"


def fmt_plain(vm_names, workers, table) -> str:
    col_w = max(len(str(w)) + 2 for w in workers)
    col_w = max(col_w, 10)
    vm_w  = max(len(v) for v in vm_names)

    header = f"{'VM':<{vm_w}}" + "".join(f"  {w:>{col_w}}" for w in workers)
    sep    = "-" * len(header)
    rows   = [header, sep]
    for vm in vm_names:
        row = f"{vm:<{vm_w}}" + "".join(
            f"  {_cell(table, vm, w):>{col_w}}" for w in workers
        )
        rows.append(row)
    return "\n".join(rows)


def fmt_csv(vm_names, workers, table) -> str:
    lines = ["VM," + ",".join(str(w) for w in workers)]
    for vm in vm_names:
        vals = ",".join(_cell(table, vm, w) for w in workers)
        lines.append(f"{vm},{vals}")
    return "\n".join(lines)


def fmt_markdown(vm_names, workers, table) -> str:
    header = "| VM | " + " | ".join(str(w) for w in workers) + " |"
    sep    = "|:---|" + "|".join("---:" for _ in workers) + "|"
    rows   = [header, sep]
    for vm in vm_names:
        vals = " | ".join(_cell(table, vm, w) for w in workers)
        rows.append(f"| {vm} | {vals} |")
    return "\n".join(rows)


def fmt_latex(vm_names, workers, table) -> str:
    cols = "l" + "r" * len(workers)
    lines = [
        r"\begin{tabular}{" + cols + "}",
        r"\toprule",
        "VM & " + " & ".join(str(w) for w in workers) + r" \\",
        r"\midrule",
    ]
    for vm in vm_names:
        vals = " & ".join(_cell(table, vm, w) for w in workers)
        lines.append(f"{vm} & {vals} \\\\")
    lines += [r"\bottomrule", r"\end{tabular}"]
    return "\n".join(lines)


FORMATTERS = {
    "plain":    fmt_plain,
    "csv":      fmt_csv,
    "markdown": fmt_markdown,
    "latex":    fmt_latex,
}

# ── main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--base-dir", default=None,
                        help="Root of the ZEBRA repo (auto-detected if omitted)")
    parser.add_argument("--format", choices=list(FORMATTERS), default="plain",
                        help="Output format (default: plain)")
    parser.add_argument("--out", default=None,
                        help="Write output to this file instead of stdout")
    args = parser.parse_args()

    base_dir = args.base_dir or find_repo_root(
        os.path.dirname(os.path.abspath(__file__))
    )

    vm_names, workers, table = build_table(base_dir)

    if not workers:
        print("No worker_sweep data found under examples/*/report/worker_sweep/",
              file=sys.stderr)
        sys.exit(1)

    output = FORMATTERS[args.format](vm_names, workers, table)

    if args.out:
        os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
        with open(args.out, "w") as fh:
            fh.write(output + "\n")
        print(f"Saved: {args.out}")
    else:
        print(output)


if __name__ == "__main__":
    main()
