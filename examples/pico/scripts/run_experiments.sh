#!/usr/bin/env bash
# ZEBRA Pico Experiment Runner
#
# Experiment 1 – Worker sweep    : vary num_workers = {8,6,4,2,1} (high → low)
#                                  with --range-interval 0 (single-point)
#                                  Opcodes that fail at more workers are
#                                  automatically skipped at all fewer workers.
# Experiment 2 – Range sweep     : vary --range-interval = {0,1,7,31,127}
#                                  with fixed num_workers = 1 (bb method)
#                                  Opcodes that fail at a smaller range are
#                                  automatically skipped at all larger ranges.
# Experiment 3 – SMT range sweep : same range sweep but with --method z3
#                                  (SMT range-verification mode); each free
#                                  input column is given Any(lo, lo+R) so the
#                                  solver enumerates every solution in the
#                                  region.  Same per-(chip,opcode) early-stop
#                                  policy as experiment 2.
#
# Results are saved under:
#   report/
#     worker_sweep/<chip>/workers_<N>/<OPCODE>.yaml
#     range_sweep/<chip>/range_<R>/<OPCODE>.yaml
#     smt_range_sweep/<chip>/range_<R>/<OPCODE>.yaml
#
# Usage (run from examples/pico/):
#   bash scripts/run_experiments.sh [--num-trial N] [--timeout-ms T]
#                                   [--workers W]
#                                   [--only-worker-sweep]
#                                   [--only-range-sweep]
#                                   [--only-smt-range-sweep]

set -euo pipefail

# ── defaults ──────────────────────────────────────────────────────────────────
NUM_TRIALS=5
TIMEOUT_MS=10000000
MAX_EXPANSIONS=30000000
FIXED_WORKERS=4      # num_workers inside each range-sweep experiment
BASE_SEED=41
PARALLEL_JOBS=1      # number of experiments to run simultaneously
RUN_WORKER_SWEEP=1
RUN_RANGE_SWEEP=1
RUN_SMT_RANGE_SWEEP=1

# ── argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --num-trial)             NUM_TRIALS="$2";    shift 2 ;;
        --timeout-ms)            TIMEOUT_MS="$2";    shift 2 ;;
        --workers)               PARALLEL_JOBS="$2"; shift 2 ;;
        --only-worker-sweep)     RUN_RANGE_SWEEP=0;  RUN_SMT_RANGE_SWEEP=0; shift ;;
        --only-range-sweep)      RUN_WORKER_SWEEP=0; RUN_SMT_RANGE_SWEEP=0; shift ;;
        --only-smt-range-sweep)  RUN_WORKER_SWEEP=0; RUN_RANGE_SWEEP=0;     shift ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── paths ─────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$VM_DIR/report"
BIN_DIR="$VM_DIR/target/release/examples"

# ── chip → opcode list ────────────────────────────────────────────────────────
# sll/sr binaries ignore --opcode-str; the label is used only for filenames.
declare -A CHIP_OPCODES
CHIP_OPCODES[addsub]="ADD SUB"
CHIP_OPCODES[bitwise]="AND OR XOR"
CHIP_OPCODES[mul]="MUL MULH MULHU MULHSU"
CHIP_OPCODES[divrem]="DIV DIVU REM REMU"
CHIP_OPCODES[lessthan]="SLT SLTU"
CHIP_OPCODES[sll]="SLL"
CHIP_OPCODES[sr]="SRL"
CHIP_OPCODES[memoryreadwrite]="LB LBU LH LHU LW SB SH SW"

CHIPS=(addsub bitwise mul divrem lessthan sll sr memoryreadwrite)

# ── experiment parameters ─────────────────────────────────────────────────────
WORKER_COUNTS=(4 3 2 1)
RANGE_INTERVALS=(0 1 7 31 127)
# Range list for SMT range-verification mode (experiment 3).  Same intervals as
# the bb sweep so the two methods can be plotted on the same axis; opcodes
# that time out at large ranges are automatically skipped via early-stop.
SMT_RANGE_INTERVALS=(0 1 7 31 127)

# ── parallel job pool ─────────────────────────────────────────────────────────
_PIDS=()

_wait_one() {
    wait "${_PIDS[0]}" 2>/dev/null || true
    _PIDS=("${_PIDS[@]:1}")
}

submit_job() {
    local max_jobs="$1"; shift
    while [[ ${#_PIDS[@]} -ge "$max_jobs" ]]; do
        _wait_one
    done
    "$@" &
    _PIDS+=($!)
}

wait_all() {
    while [[ ${#_PIDS[@]} -gt 0 ]]; do
        _wait_one
    done
}

# ── range-level failure tracking ──────────────────────────────────────────────
# Each failed (chip, opcode) writes a marker so the next range level can skip it.
# Uses a temp dir so background subprocesses can signal failures to the parent.
FAILED_DIR="$(mktemp -d /tmp/zebra_failed_XXXXXX)"
trap 'rm -rf "$FAILED_DIR"' EXIT

is_failed()    { [[ -f "$FAILED_DIR/${1}__${2}" ]]; }
mark_failed()  { touch "$FAILED_DIR/${1}__${2}";   }
reset_failed() { rm -f "$FAILED_DIR"/* 2>/dev/null || true; }

# ── helpers ───────────────────────────────────────────────────────────────────

make_config() {
    local workers=$1 seed=$2
    local tmpfile
    tmpfile="$(mktemp /tmp/zebra_config_XXXXXX.json)"
    cat > "$tmpfile" <<JSON
{
    "time_out_ms": ${TIMEOUT_MS},
    "max_expansions": ${MAX_EXPANSIONS},
    "num_workers": ${workers},
    "seed": ${seed}
}
JSON
    echo "$tmpfile"
}

# run_one <chip> <opcode> <workers> <range> <outfile> [track_failure=0] [method=bb]
# track_failure=1: mark (chip,opcode) as failed so larger ranges are skipped.
# method=bb (default) or z3 (SMT range-verification mode).
run_one() {
    local chip="$1" opcode="$2" workers="$3" range="$4" outfile="$5" track="${6:-0}" method="${7:-bb}"
    mkdir -p "$(dirname "$outfile")"
    local trial_config ratio
    trial_config="$(make_config "$workers" "$BASE_SEED")"
    echo "    run: chip=$chip opcode=$opcode method=$method workers=$workers range=$range trials=$NUM_TRIALS"
    (cd "$VM_DIR" && \
        "$BIN_DIR/$chip" \
            --config         "$trial_config" \
            --opcode-str     "$opcode" \
            --method         "$method" \
            --num-trial      "$NUM_TRIALS" \
            --range-interval "$range" \
            --ouptput-path   "$outfile" \
            --turn-off-ui \
        2>/dev/null
    ) || true
    rm -f "$trial_config"

    ratio=$(grep -m1 'success_ratio:' "$outfile" 2>/dev/null | awk '{print $2}')
    if [[ -z "$ratio" ]] || ! awk "BEGIN { exit ($ratio >= 1.0) ? 0 : 1 }"; then
        echo "      -> not verified (success_ratio=${ratio:-N/A})"
        if [[ "$track" == "1" ]]; then
            mark_failed "$chip" "$opcode"
        fi
    fi
}

# ── experiment 1: worker sweep ────────────────────────────────────────────────
if [[ $RUN_WORKER_SWEEP -eq 1 ]]; then
    echo "======================================================="
    echo " Experiment 1: Worker Sweep  (range-interval=0)"
    echo "  workers : ${WORKER_COUNTS[*]}"
    echo "  chips   : ${CHIPS[*]}"
    echo "  trials  : $NUM_TRIALS"
    echo "  parallel: $PARALLEL_JOBS job(s)"
    echo "  note    : opcodes that fail at W workers are skipped for all W' < W"
    echo "======================================================="

    reset_failed
    for workers in "${WORKER_COUNTS[@]}"; do
        echo ""
        echo "--- workers=$workers ---"
        local_skip=0
        for chip in "${CHIPS[@]}"; do
            for opcode in ${CHIP_OPCODES[$chip]}; do
                if is_failed "$chip" "$opcode"; then
                    echo "  [SKIP] $chip/$opcode (failed at more workers)"
                    (( local_skip++ )) || true
                    continue
                fi
                outfile="$RESULTS_DIR/worker_sweep/$chip/workers_${workers}/${opcode}.yaml"
                submit_job "$PARALLEL_JOBS" run_one \
                    "$chip" "$opcode" "$workers" 0 "$outfile" 1
            done
        done
        wait_all
        echo "  (skipped $local_skip opcode(s) due to prior worker failure)"
    done
    echo ""
    echo "Worker sweep complete."
fi

# ── experiment 2: range sweep ─────────────────────────────────────────────────
if [[ $RUN_RANGE_SWEEP -eq 1 ]]; then
    reset_failed  # worker-sweep failures must not carry over
    echo ""
    echo "======================================================="
    echo " Experiment 2: Range Sweep  (num_workers=$FIXED_WORKERS)"
    echo "  ranges  : ${RANGE_INTERVALS[*]}"
    echo "  chips   : ${CHIPS[*]}"
    echo "  trials  : $NUM_TRIALS"
    echo "  parallel: $PARALLEL_JOBS job(s)"
    echo "  note    : opcodes that fail at range R are skipped for all R' > R"
    echo "======================================================="

    for range in "${RANGE_INTERVALS[@]}"; do
        echo ""
        echo "--- range-interval=$range ---"
        local_skip=0
        for chip in "${CHIPS[@]}"; do
            for opcode in ${CHIP_OPCODES[$chip]}; do
                if is_failed "$chip" "$opcode"; then
                    echo "  [SKIP] $chip/$opcode (failed at smaller range)"
                    (( local_skip++ )) || true
                    continue
                fi
                outfile="$RESULTS_DIR/range_sweep/$chip/range_${range}/${opcode}.yaml"
                submit_job "$PARALLEL_JOBS" run_one \
                    "$chip" "$opcode" "$FIXED_WORKERS" "$range" "$outfile" 1
            done
        done
        wait_all
        echo "  (skipped $local_skip opcode(s) due to prior range failure)"
    done
    echo ""
    echo "Range sweep complete."
fi

# ── experiment 3: smt range sweep ─────────────────────────────────────────────
if [[ $RUN_SMT_RANGE_SWEEP -eq 1 ]]; then
    reset_failed  # prior sweep failures must not carry over
    echo ""
    echo "======================================================="
    echo " Experiment 3: SMT Range Sweep  (method=z3, num_workers=1)"
    echo "  ranges  : ${SMT_RANGE_INTERVALS[*]}"
    echo "  chips   : ${CHIPS[*]}"
    echo "  trials  : $NUM_TRIALS"
    echo "  parallel: $PARALLEL_JOBS job(s)"
    echo "  note    : opcodes that fail at range R are skipped for all R' > R"
    echo "======================================================="

    for range in "${SMT_RANGE_INTERVALS[@]}"; do
        echo ""
        echo "--- smt range-interval=$range ---"
        local_skip=0
        for chip in "${CHIPS[@]}"; do
            for opcode in ${CHIP_OPCODES[$chip]}; do
                if is_failed "$chip" "$opcode"; then
                    echo "  [SKIP] $chip/$opcode (failed at smaller smt range)"
                    (( local_skip++ )) || true
                    continue
                fi
                outfile="$RESULTS_DIR/smt_range_sweep/$chip/range_${range}/${opcode}.yaml"
                submit_job "$PARALLEL_JOBS" run_one \
                    "$chip" "$opcode" 1 "$range" "$outfile" 1 "z3"
            done
        done
        wait_all
        echo "  (skipped $local_skip opcode(s) due to prior smt range failure)"
    done
    echo ""
    echo "SMT range sweep complete."
fi

echo ""
echo "All experiments done. Results in: $RESULTS_DIR"
echo "Run:  bash scripts/aggregate.sh  to summarise."
