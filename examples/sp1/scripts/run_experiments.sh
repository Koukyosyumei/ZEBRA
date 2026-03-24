#!/usr/bin/env bash
# ZEBRA SP1 Experiment Runner
#
# Experiment 1 – Worker sweep  : vary num_workers = {1,2,3,8}
#                                with --range-interval 0 (single-point)
# Experiment 2 – Range sweep   : vary --range-interval = {0,1,3,7,15,31,63,127}
#                                with fixed num_workers = 8
#
# Results are saved under:
#   report/
#     worker_sweep/<chip>/workers_<N>/<OPCODE>.yaml
#     range_sweep/<chip>/range_<R>/<OPCODE>.yaml
#
# Usage (run from examples/sp1/):
#   bash scripts/run_experiments.sh [--num-trial N] [--timeout-ms T]
#   bash scripts/run_experiments.sh --only-worker-sweep
#   bash scripts/run_experiments.sh --only-range-sweep

set -euo pipefail

# ── defaults ─────────────────────────────────────────────────────────────────
NUM_TRIALS=5
TIMEOUT_MS=1000000
MAX_EXPANSIONS=30000000
FIXED_WORKERS=8
RUN_WORKER_SWEEP=1
RUN_RANGE_SWEEP=1

# ── argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --num-trial)         NUM_TRIALS="$2";    shift 2 ;;
        --timeout-ms)        TIMEOUT_MS="$2";    shift 2 ;;
        --only-worker-sweep) RUN_RANGE_SWEEP=0;  shift ;;
        --only-range-sweep)  RUN_WORKER_SWEEP=0; shift ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── paths ─────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$VM_DIR/report"
BIN_DIR="$VM_DIR/target/release/examples"

# ── chip → opcode list ────────────────────────────────────────────────────────
# shiftleft ignores --opcode-str; "SLL" is used only as a filename label.
declare -A CHIP_OPCODES
CHIP_OPCODES[addsub]="ADD SUB"
CHIP_OPCODES[bitwise]="AND OR XOR"
CHIP_OPCODES[mul]="MUL MULH MULHU MULHSU"
CHIP_OPCODES[jump]="JAL JALR"
CHIP_OPCODES[shiftleft]="SLL"
CHIP_OPCODES[branch]="BEQ BGE BLT BNE"

CHIPS=(addsub bitwise mul jump shiftleft branch)

# ── experiment parameters ─────────────────────────────────────────────────────
WORKER_COUNTS=(1 2 3 8)
RANGE_INTERVALS=(0 1 3 7 15 31 63 127)

# ── helpers ───────────────────────────────────────────────────────────────────

make_config() {
    local workers=$1
    local tmpfile
    tmpfile="$(mktemp /tmp/zebra_config_XXXXXX.json)"
    cat > "$tmpfile" <<JSON
{
    "time_out_ms": ${TIMEOUT_MS},
    "max_expansions": ${MAX_EXPANSIONS},
    "num_workers": ${workers}
}
JSON
    echo "$tmpfile"
}

run_one() {
    local chip="$1" opcode="$2" config="$3" range="$4" outfile="$5"
    mkdir -p "$(dirname "$outfile")"
    echo "    run: chip=$chip opcode=$opcode range=$range -> $(basename "$outfile")"
    (cd "$VM_DIR" && \
        "$BIN_DIR/$chip" \
            --config        "$config" \
            --opcode-str    "$opcode" \
            --method        "bb" \
            --num-trial     "$NUM_TRIALS" \
            --range-interval "$range" \
            --ouptput-path  "$outfile" \
        2>/dev/null
    )
}

# ── experiment 1: worker sweep ────────────────────────────────────────────────
if [[ $RUN_WORKER_SWEEP -eq 1 ]]; then
    echo "======================================================="
    echo " Experiment 1: Worker Sweep  (range-interval=0)"
    echo "  workers : ${WORKER_COUNTS[*]}"
    echo "  chips   : ${CHIPS[*]}"
    echo "  trials  : $NUM_TRIALS"
    echo "======================================================="

    for workers in "${WORKER_COUNTS[@]}"; do
        echo ""
        echo "--- workers=$workers ---"
        config="$(make_config "$workers")"
        trap "rm -f '$config'" EXIT

        for chip in "${CHIPS[@]}"; do
            for opcode in ${CHIP_OPCODES[$chip]}; do
                outfile="$RESULTS_DIR/worker_sweep/$chip/workers_${workers}/${opcode}.yaml"
                run_one "$chip" "$opcode" "$config" 0 "$outfile"
            done
        done

        rm -f "$config"
        trap - EXIT
    done
    echo ""
    echo "Worker sweep complete."
fi

# ── experiment 2: range sweep ─────────────────────────────────────────────────
if [[ $RUN_RANGE_SWEEP -eq 1 ]]; then
    echo ""
    echo "======================================================="
    echo " Experiment 2: Range Sweep  (workers=$FIXED_WORKERS)"
    echo "  ranges  : ${RANGE_INTERVALS[*]}"
    echo "  chips   : ${CHIPS[*]}"
    echo "  trials  : $NUM_TRIALS"
    echo "======================================================="

    config="$(make_config "$FIXED_WORKERS")"
    trap "rm -f '$config'" EXIT

    for range in "${RANGE_INTERVALS[@]}"; do
        echo ""
        echo "--- range-interval=$range ---"
        for chip in "${CHIPS[@]}"; do
            for opcode in ${CHIP_OPCODES[$chip]}; do
                outfile="$RESULTS_DIR/range_sweep/$chip/range_${range}/${opcode}.yaml"
                run_one "$chip" "$opcode" "$config" "$range" "$outfile"
            done
        done
    done

    rm -f "$config"
    trap - EXIT
    echo ""
    echo "Range sweep complete."
fi

echo ""
echo "All experiments done. Results in: $RESULTS_DIR"
echo "Run:  bash scripts/aggregate.sh  to summarise."
