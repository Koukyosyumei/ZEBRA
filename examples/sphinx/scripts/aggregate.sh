#!/usr/bin/env bash
# ZEBRA SP1 Experiment Aggregator
#
# Reads the experiment results produced by run_experiments.sh and outputs:
#   1. A human-readable chip-level summary (stdout)
#   2. Two CSV files:
#        report/worker_sweep_summary.csv
#        report/range_sweep_summary.csv
#
# Aggregation is chip-level: all opcodes within a chip are averaged.
#
# Usage (run from examples/sp1/):
#   bash scripts/aggregate.sh [results_dir]   (default: report/)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="${1:-$VM_DIR/report}"

CHIPS=(addsub bitwise mul jump shiftleft)

if [[ ! -d "$RESULTS_DIR" ]]; then
    echo "Results directory not found: $RESULTS_DIR"
    echo "Run scripts/run_experiments.sh first."
    exit 1
fi

# ── embedded Python: aggregate all *.yaml in a directory ─────────────────────
agg_dir() {
    python3 - "$1" <<'PYEOF'
import sys, os, glob, yaml

folder = sys.argv[1]
yamls = sorted(glob.glob(os.path.join(folder, "*.yaml")))

if not yamls:
    print("0 nan nan nan nan")
    sys.exit(0)

success_list, time_list, var_list, area_list = [], [], [], []

for path in yamls:
    with open(path) as fh:
        try:
            data = yaml.safe_load(fh)
        except Exception:
            continue
    if not data:
        continue
    rep = data.get("report", {})
    if "success_ratio"     in rep: success_list.append(rep["success_ratio"])
    if "exe_time_mean"     in rep: time_list.append(rep["exe_time_mean"])
    if "exe_time_variance" in rep: var_list.append(rep["exe_time_variance"])
    if "area"              in rep: area_list.append(rep["area"])

def avg(lst): return sum(lst) / len(lst) if lst else float("nan")

n       = len(yamls)
success = avg(success_list)
t_mean  = avg(time_list)
t_var   = avg(var_list)
area    = avg(area_list) if area_list else "N/A"

def fmt(v):
    if isinstance(v, float): return f"{v:.6f}"
    return str(v)

print(f"{n} {fmt(success)} {fmt(t_mean)} {fmt(t_var)} {fmt(area)}")
PYEOF
}

# ── table formatting ──────────────────────────────────────────────────────────
print_row() {
    printf "  %-12s  %-14s  %-8s  %-8s  %-14s  %-14s  %s\n" \
        "$1" "$2" "$3" "$4" "$5" "$6" "$7"
}
print_header() {
    print_row "chip" "$1" "opcodes" "success" "time_mean(s)" "time_variance" "area"
    printf "  %s\n" "$(printf '%0.s-' {1..95})"
}

# ── worker sweep ──────────────────────────────────────────────────────────────
WORKER_DIR="$RESULTS_DIR/worker_sweep"
CSV_WORKER="$RESULTS_DIR/worker_sweep_summary.csv"

if [[ -d "$WORKER_DIR" ]]; then
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║          SP1 – EXPERIMENT 1: WORKER SWEEP SUMMARY              ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"

    echo "chip,workers,n_opcodes,success_ratio,exe_time_mean,exe_time_variance,area" > "$CSV_WORKER"

    for chip in "${CHIPS[@]}"; do
        chipdir="$WORKER_DIR/$chip"
        [[ -d "$chipdir" ]] || continue
        echo ""
        print_header "workers"
        for wdir in $(ls "$chipdir" | grep '^workers_' | sort -t_ -k2 -n); do
            workers="${wdir#workers_}"
            read -r n_op success t_mean t_var area <<< "$(agg_dir "$chipdir/$wdir")"
            print_row "$chip" "workers=$workers" "$n_op" "$success" "$t_mean" "$t_var" "$area"
            echo "$chip,$workers,$n_op,$success,$t_mean,$t_var,$area" >> "$CSV_WORKER"
        done
    done

    echo ""
    echo "  CSV saved: $CSV_WORKER"
fi

# ── range sweep ───────────────────────────────────────────────────────────────
RANGE_DIR="$RESULTS_DIR/range_sweep"
CSV_RANGE="$RESULTS_DIR/range_sweep_summary.csv"

if [[ -d "$RANGE_DIR" ]]; then
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║          SP1 – EXPERIMENT 2: RANGE SWEEP SUMMARY               ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"

    echo "chip,range_interval,n_opcodes,success_ratio,exe_time_mean,exe_time_variance,area" > "$CSV_RANGE"

    for chip in "${CHIPS[@]}"; do
        chipdir="$RANGE_DIR/$chip"
        [[ -d "$chipdir" ]] || continue
        echo ""
        print_header "range"
        for rdir in $(ls "$chipdir" | grep '^range_' | sort -t_ -k2 -n); do
            range="${rdir#range_}"
            read -r n_op success t_mean t_var area <<< "$(agg_dir "$chipdir/$rdir")"
            print_row "$chip" "range=$range" "$n_op" "$success" "$t_mean" "$t_var" "$area"
            echo "$chip,$range,$n_op,$success,$t_mean,$t_var,$area" >> "$CSV_RANGE"
        done
    done

    echo ""
    echo "  CSV saved: $CSV_RANGE"
fi

echo ""
echo "Aggregation complete."
