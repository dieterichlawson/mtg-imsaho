#!/usr/bin/env bash
# Invariant-checked fuzzing campaign: seeded random-vs-random games with
# mtg_engine::invariants checked at every decision point, plus a replay
# determinism spot check.
#
# Usage: scripts/fuzz.sh [GAMES_PER_PAIR] [START_SEED]
#   GAMES_PER_PAIR  seeded games per deck pairing (default 100)
#   START_SEED      first seed (default 1); seeds run consecutively
#   FUZZ_DECKS      space-separated deck files to pair up instead of the
#                   default set (e.g. FUZZ_DECKS="decks/gw-humans.txt ...")
#   FUZZ_JOBS       parallel games (default: number of CPUs)
#
# The default deck set is decks/coverage/ — ten decks that together contain
# every castable card the engine implements (pinned by
# mtg-engine/tests/deck_coverage.rs), so the campaign can reach every card.
#
# Exit code 0 = every game finished clean. Failing games leave their output
# in logs/fuzz-<date>/ and are summarized at the end; a failure replays with:
#   target/release/mtg-runner --p1 random --p2 random \
#     --deck1 <d1> --deck2 <d2> --seed <seed> --check-invariants
set -u
cd "$(dirname "$0")/.."

GAMES="${1:-100}"
START="${2:-1}"
JOBS="${FUZZ_JOBS:-$(nproc 2>/dev/null || echo 2)}"
RUNNER=target/release/mtg-runner
OUT="logs/fuzz-$(date +%Y%m%d-%H%M%S)"

cargo build --release -p mtg-runner || exit 1
mkdir -p "$OUT"

if [ -n "${FUZZ_DECKS:-}" ]; then
  # shellcheck disable=SC2206 -- word-splitting the list is the interface
  DECKS=($FUZZ_DECKS)
else
  DECKS=(decks/coverage/*.txt)
fi

# Every game as one job line: "deck1 deck2 seed pair-name". Games are
# independent, so they fan out over $JOBS workers; a failing game keeps its
# log in $OUT (a passing one deletes it), which is also how failures are
# counted across workers.
jobs_file="$OUT/.jobs"
for ((i = 0; i < ${#DECKS[@]}; i++)); do
  for ((j = i; j < ${#DECKS[@]}; j++)); do
    d1="${DECKS[$i]}"; d2="${DECKS[$j]}"
    pair="$(basename "$d1" .txt)-vs-$(basename "$d2" .txt)"
    for ((s = START; s < START + GAMES; s++)); do
      printf '%s %s %s %s\n' "$d1" "$d2" "$s" "$pair"
    done
  done
done > "$jobs_file"
total=$(wc -l < "$jobs_file")

export RUNNER OUT
xargs -P "$JOBS" -n 4 bash -c '
  d1=$0; d2=$1; s=$2; pair=$3
  log="$OUT/$pair-seed$s.txt"
  if ! "$RUNNER" --p1 random --p2 random --deck1 "$d1" --deck2 "$d2" \
      --seed "$s" --check-invariants --quiet > "$log" 2>&1; then
    echo "FAIL: $pair seed $s (log: $log)"
  else
    rm -f "$log"
  fi
' < "$jobs_file"
rm -f "$jobs_file"

# Replay determinism spot check: the first seed of each pairing, run twice.
for ((i = 0; i < ${#DECKS[@]}; i++)); do
  for ((j = i; j < ${#DECKS[@]}; j++)); do
    d1="${DECKS[$i]}"; d2="${DECKS[$j]}"
    pair="$(basename "$d1" .txt)-vs-$(basename "$d2" .txt)"
    "$RUNNER" --p1 random --p2 random --deck1 "$d1" --deck2 "$d2" \
        --seed "$START" --quiet > "$OUT/det-a.txt" 2>&1
    "$RUNNER" --p1 random --p2 random --deck1 "$d1" --deck2 "$d2" \
        --seed "$START" --quiet > "$OUT/det-b.txt" 2>&1
    if ! diff -q "$OUT/det-a.txt" "$OUT/det-b.txt" > /dev/null; then
      cp "$OUT/det-a.txt" "$OUT/$pair-seed$START-replay-a.txt"
      cp "$OUT/det-b.txt" "$OUT/$pair-seed$START-replay-b.txt"
      echo "FAIL: $pair seed $START is not replay-deterministic"
    fi
  done
done
rm -f "$OUT/det-a.txt" "$OUT/det-b.txt"

failures=$(find "$OUT" -name '*.txt' | wc -l)
echo
echo "fuzz: $total games, $failures failures ($JOBS workers)"
if [ "$failures" -eq 0 ]; then
  rmdir "$OUT" 2>/dev/null
  exit 0
fi
echo "failure logs kept in $OUT/"
exit 1
