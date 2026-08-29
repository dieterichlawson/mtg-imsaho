#!/usr/bin/env bash
# Invariant-checked fuzzing campaign: seeded random-vs-random games with
# mtg_engine::invariants checked at every decision point, plus a replay
# determinism spot check.
#
# Usage: scripts/fuzz.sh [GAMES_PER_PAIR] [START_SEED]
#   GAMES_PER_PAIR  seeded games per deck pairing (default 100)
#   START_SEED      first seed (default 1); seeds run consecutively
#
# Exit code 0 = every game finished clean. Failing games leave their output
# in logs/fuzz-<date>/ and are summarized at the end; a failure replays with:
#   target/release/mtg-runner --p1 random --p2 random \
#     --deck1 <d1> --deck2 <d2> --seed <seed> --check-invariants
set -u
cd "$(dirname "$0")/.."

GAMES="${1:-100}"
START="${2:-1}"
RUNNER=target/release/mtg-runner
OUT="logs/fuzz-$(date +%Y%m%d-%H%M%S)"

cargo build --release -p mtg-runner || exit 1
mkdir -p "$OUT"

DECKS=(decks/gw-humans.txt decks/rb-vampires.txt decks/ub-zombies.txt decks/ug-spider-spawning.txt)

total=0
failures=0

# Every unordered deck pairing, including mirrors.
for ((i = 0; i < ${#DECKS[@]}; i++)); do
  for ((j = i; j < ${#DECKS[@]}; j++)); do
    d1="${DECKS[$i]}"; d2="${DECKS[$j]}"
    pair="$(basename "$d1" .txt)-vs-$(basename "$d2" .txt)"
    for ((s = START; s < START + GAMES; s++)); do
      total=$((total + 1))
      log="$OUT/$pair-seed$s.txt"
      if ! "$RUNNER" --p1 random --p2 random --deck1 "$d1" --deck2 "$d2" \
          --seed "$s" --check-invariants --quiet > "$log" 2>&1; then
        failures=$((failures + 1))
        echo "FAIL: $pair seed $s (log: $log)"
      else
        rm -f "$log"
      fi
    done
  done
done

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
      failures=$((failures + 1))
      cp "$OUT/det-a.txt" "$OUT/$pair-seed$START-replay-a.txt"
      cp "$OUT/det-b.txt" "$OUT/$pair-seed$START-replay-b.txt"
      echo "FAIL: $pair seed $START is not replay-deterministic"
    fi
  done
done
rm -f "$OUT/det-a.txt" "$OUT/det-b.txt"

echo
echo "fuzz: $total games, $failures failures"
if [ "$failures" -eq 0 ]; then
  rmdir "$OUT" 2>/dev/null
  exit 0
fi
echo "failure logs kept in $OUT/"
exit 1
