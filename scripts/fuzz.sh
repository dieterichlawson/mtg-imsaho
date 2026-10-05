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
#   FUZZ_GAME_TIMEOUT  seconds one game may run before it is a finding
#                   (default 300). The slowest coverage game takes under
#                   2 s; a game still going at the limit is either a
#                   performance cliff or a loop the watchdog cannot see,
#                   and a game with no limit held its worker until the
#                   job's 90-minute cap cancelled the shard — which files
#                   nothing, so it surfaced with no seed attached (#644).
#   FUZZ_RUNNER     runner binary to use instead of building the release
#                   one (the script's own test drives it with a stub)
#   FUZZ_EXTRA_GAMES  games per pairing of the doubles decks (decks/fuzz/
#                   *-doubles.txt; default GAMES/10, at least 1; 0 skips)
#   FUZZ_FLOOD_GAMES  seeds of the flood mirror (decks/fuzz/flood/*.txt;
#                   default 3; 0 skips)
#
# The default deck set is decks/coverage/ — ten decks that together contain
# every castable card the engine implements (pinned by
# mtg-engine/tests/deck_coverage.rs), so the campaign can reach every card.
# Two smaller campaigns follow it unless FUZZ_DECKS names a set: the doubles
# decks (every coverage deck with each spell twice, so a board can hold two
# same-named objects — the byte-identical-row class, #612/#631, is invisible
# to singleton decks) and the flood mirror (a token engine, so the board gets
# wider than any coverage game makes it; the performance walls of #565,
# #640-#642 were all found by hand on boards the fuzz never built).
#
# Every game also writes --decision-stats, and scripts/fuzz_reach.py sums
# them into logs/fuzz-reach-<date>.md: which kinds of question the seats
# were asked, and which kinds the engine defines that no game reached. The
# invariant checker is silent about a prompt the random seat never gets to
# (#664-#667 were found by hand, with the fuzz green every night), so the
# campaign says what it did not cover.
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
GAME_TIMEOUT="${FUZZ_GAME_TIMEOUT:-300}"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="logs/fuzz-$STAMP"
REACH="logs/fuzz-reach-$STAMP.md"
EXTRA_GAMES="${FUZZ_EXTRA_GAMES:-$(( GAMES / 10 > 0 ? GAMES / 10 : 1 ))}"
FLOOD_GAMES="${FUZZ_FLOOD_GAMES:-3}"

if [ -n "${FUZZ_RUNNER:-}" ]; then
  RUNNER="$FUZZ_RUNNER"
else
  RUNNER=target/release/mtg-runner
  cargo build --release -p mtg-runner || exit 1
fi
mkdir -p "$OUT/stats"

EXTRA=()
FLOOD=()
if [ -n "${FUZZ_DECKS:-}" ]; then
  # shellcheck disable=SC2206 -- word-splitting the list is the interface
  DECKS=($FUZZ_DECKS)
else
  DECKS=(decks/coverage/*.txt)
  [ "$EXTRA_GAMES" -gt 0 ] && EXTRA=(decks/fuzz/*-doubles.txt)
  [ "$FLOOD_GAMES" -gt 0 ] && FLOOD=(decks/fuzz/flood/*.txt)
fi

# Every game as one job line: "deck1 deck2 seed pair-name". Games are
# independent, so they fan out over $JOBS workers; a failing game keeps its
# log in $OUT (a passing one deletes it), which is also how failures are
# counted across workers.
jobs_file="$OUT/.jobs"
# All pairings (i <= j) of a deck set, N seeds each.
emit_pairings() {
  local games=$1; shift
  local set=("$@")
  for ((i = 0; i < ${#set[@]}; i++)); do
    for ((j = i; j < ${#set[@]}; j++)); do
      d1="${set[$i]}"; d2="${set[$j]}"
      pair="$(basename "$d1" .txt)-vs-$(basename "$d2" .txt)"
      for ((s = START; s < START + games; s++)); do
        printf '%s %s %s %s\n' "$d1" "$d2" "$s" "$pair"
      done
    done
  done
}
{
  emit_pairings "$GAMES" "${DECKS[@]}"
  [ "${#EXTRA[@]}" -gt 0 ] && emit_pairings "$EXTRA_GAMES" "${EXTRA[@]}"
  # The flood deck is a mirror: wide boards on both sides.
  for d in "${FLOOD[@]}"; do
    pair="$(basename "$d" .txt)-vs-$(basename "$d" .txt)"
    for ((s = START; s < START + FLOOD_GAMES; s++)); do
      printf '%s %s %s %s\n' "$d" "$d" "$s" "$pair"
    done
  done
} > "$jobs_file"
total=$(wc -l < "$jobs_file")

export RUNNER OUT GAME_TIMEOUT
xargs -P "$JOBS" -n 4 bash -c '
  d1=$0; d2=$1; s=$2; pair=$3
  log="$OUT/$pair-seed$s.txt"
  timeout --kill-after=10 "$GAME_TIMEOUT" "$RUNNER" --p1 random --p2 random \
      --deck1 "$d1" --deck2 "$d2" --seed "$s" --check-invariants --quiet \
      --decision-stats "$OUT/stats/$pair-seed$s.json" > "$log" 2>&1
  status=$?
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    # Said in the log, not only here: a --quiet game that is killed has
    # written nothing, and the filing step skips an empty log.
    echo "TIMEOUT: the game was still running after ${GAME_TIMEOUT}s and was killed" >> "$log"
    echo "FAIL: $pair seed $s timed out after ${GAME_TIMEOUT}s (log: $log)"
  elif [ "$status" -ne 0 ]; then
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
    # Bounded too: the same game as above, run twice more.
    timeout --kill-after=10 "$GAME_TIMEOUT" "$RUNNER" --p1 random --p2 random \
        --deck1 "$d1" --deck2 "$d2" --seed "$START" --quiet > "$OUT/det-a.txt" 2>&1
    timeout --kill-after=10 "$GAME_TIMEOUT" "$RUNNER" --p1 random --p2 random \
        --deck1 "$d1" --deck2 "$d2" --seed "$START" --quiet > "$OUT/det-b.txt" 2>&1
    if ! diff -q "$OUT/det-a.txt" "$OUT/det-b.txt" > /dev/null; then
      cp "$OUT/det-a.txt" "$OUT/$pair-seed$START-replay-a.txt"
      cp "$OUT/det-b.txt" "$OUT/$pair-seed$START-replay-b.txt"
      echo "FAIL: $pair seed $START is not replay-deterministic"
    fi
  done
done
rm -f "$OUT/det-a.txt" "$OUT/det-b.txt"

# What the campaign asked, and what it never did (kept beside $OUT so a
# clean run can still remove $OUT).
if ! ls "$OUT/stats"/*.json > /dev/null 2>&1; then
  echo "no decision stats were written, so no reach report"
elif command -v python3 > /dev/null; then
  python3 scripts/fuzz_reach.py --stats "$OUT/stats" --out "$REACH" | grep -v '^NEVER REACHED:' || true
  echo "reach report: $REACH"
else
  echo "python3 not found; no reach report (the stats were in $OUT/stats)"
fi
rm -rf "$OUT/stats"

failures=$(find "$OUT" -name '*.txt' | wc -l)
echo
echo "fuzz: $total games, $failures failures ($JOBS workers)"
if [ "$failures" -eq 0 ]; then
  rmdir "$OUT" 2>/dev/null
  exit 0
fi
echo "failure logs kept in $OUT/"
exit 1
