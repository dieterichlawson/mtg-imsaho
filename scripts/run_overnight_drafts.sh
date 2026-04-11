#!/usr/bin/env bash
#
# Run many 8-seat best-of-3 Innistrad drafts in parallel with
# gemini-3.1-flash-lite-preview, high:high thinking.
#
# Usage:
#   ./scripts/run_overnight_drafts.sh            # 100 drafts, 5 parallel
#   ./scripts/run_overnight_drafts.sh 20 3       # 20 drafts, 3 parallel
#   ./scripts/run_overnight_drafts.sh 100 5 runA # custom run tag
#
# Overnight-safe: wrap this script with `caffeinate` to prevent the
# Mac from sleeping or the screensaver from kicking in:
#
#   caffeinate -d -i -s ./scripts/run_overnight_drafts.sh
#
#     -d  prevents display sleep
#     -i  prevents idle sleep
#     -s  prevents sleep when on AC power
#
# Or equivalently in nohup for full detach after login ends:
#
#   nohup caffeinate -d -i -s ./scripts/run_overnight_drafts.sh >overnight.out 2>&1 &
#
# Each draft's full structured log is saved to
#   logs/overnight-<tag>/draft-NNN.log
# Its stdout/stderr is saved alongside as
#   logs/overnight-<tag>/draft-NNN.out
# A top-level summary is written to
#   logs/overnight-<tag>/summary.txt
#
# The script exits 0 if every draft exited cleanly, 1 if any failed.

set -u
set -o pipefail

TOTAL_DRAFTS=${1:-100}
PARALLEL=${2:-5}
TAG=${3:-$(date +%Y%m%d-%H%M%S)}

MODEL="gemini:gemini-3.1-flash-lite-preview:high:high"
PLAYERS=8
BEST_OF=3
BINARY="./target/release/mtg-draft-runner"

OUT_DIR="logs/overnight-${TAG}"
mkdir -p "$OUT_DIR"

# Build release once up front so workers don't race on cargo and we
# don't burn extra cycles on debug-mode overhead during a long run.
echo "Building mtg-draft-runner (release)..."
cargo build --release -p mtg-draft-runner

echo "=== Overnight draft run ==="
echo "Tag:         $TAG"
echo "Total:       $TOTAL_DRAFTS drafts"
echo "Parallel:    $PARALLEL concurrent"
echo "Model:       $MODEL"
echo "Players:     $PLAYERS"
echo "Best of:     $BEST_OF"
echo "Out dir:     $OUT_DIR"
echo "Started:     $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================"

START_TS=$(date +%s)

# Write a summary header that we'll append to.
SUMMARY="$OUT_DIR/summary.txt"
{
    echo "# Overnight draft run $TAG"
    echo "started: $(date -Iseconds)"
    echo "total: $TOTAL_DRAFTS"
    echo "parallel: $PARALLEL"
    echo "model: $MODEL"
    echo ""
    echo "idx\tstatus\twall_seconds\tlog_file"
} > "$SUMMARY"

run_one() {
    local idx=$1
    local padded
    padded=$(printf "%03d" "$idx")
    local log_file="$OUT_DIR/draft-${padded}.log"
    local out_file="$OUT_DIR/draft-${padded}.out"
    local start
    start=$(date +%s)

    "$BINARY" \
        --players "$PLAYERS" \
        --best-of "$BEST_OF" \
        --model "$MODEL" \
        --log "$log_file" \
        --quiet \
        >"$out_file" 2>&1

    local status=$?
    local end
    end=$(date +%s)
    local elapsed=$((end - start))

    if [[ $status -eq 0 ]]; then
        printf "%d\tok\t%d\t%s\n" "$idx" "$elapsed" "$log_file" >> "$SUMMARY"
        printf "[%s] draft %03d ok (%ds)\n" "$(date '+%H:%M:%S')" "$idx" "$elapsed"
    else
        printf "%d\tFAIL(%d)\t%d\t%s\n" "$idx" "$status" "$elapsed" "$log_file" >> "$SUMMARY"
        printf "[%s] draft %03d FAILED exit=%d (%ds) — see %s\n" \
            "$(date '+%H:%M:%S')" "$idx" "$status" "$elapsed" "$out_file"
    fi
    return "$status"
}

export -f run_one
export OUT_DIR BINARY MODEL PLAYERS BEST_OF SUMMARY

# Drive the fan-out with xargs. `-P $PARALLEL` keeps exactly that many
# drafts running; as each finishes, xargs starts the next one until
# $TOTAL_DRAFTS is reached.
seq 1 "$TOTAL_DRAFTS" \
    | xargs -n 1 -P "$PARALLEL" -I {} bash -c 'run_one "$@"' _ {}

OVERALL_STATUS=$?

END_TS=$(date +%s)
ELAPSED=$((END_TS - START_TS))

OK_COUNT=$(grep -c $'\tok\t' "$SUMMARY" || true)
FAIL_COUNT=$(grep -c $'\tFAIL' "$SUMMARY" || true)

{
    echo ""
    echo "finished: $(date -Iseconds)"
    echo "elapsed_seconds: $ELAPSED"
    echo "ok: $OK_COUNT"
    echo "fail: $FAIL_COUNT"
} >> "$SUMMARY"

echo "============================"
echo "Done: $OK_COUNT ok, $FAIL_COUNT failed, ${ELAPSED}s total"
echo "Summary: $SUMMARY"

exit $OVERALL_STATUS
