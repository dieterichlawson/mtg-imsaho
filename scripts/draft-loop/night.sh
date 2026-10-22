#!/bin/sh
# One night of the Innistrad draft loop: an eight-seat draft and Swiss
# tournament on `claude -p` seats, its results in the ledger, every seat's
# lessons, and the guide rewritten from them.
#
#   scripts/draft-loop/night.sh [DATE]          # DATE defaults to today (UTC)
#
# Even seats draft with reports/draft-loop/guide.md in their prompt and odd
# seats without it, so the ledger can say whether the guide helps
# (`stats.md`, "Guided against unguided seats"). The seed is the date, so a
# night replays. Everything the loop keeps goes under reports/draft-loop/:
#
#   nights/DATE.json            the night as data (ledger.py night)
#   nights/DATE-lessons.json    each seat's lessons (reflect.py debrief)
#   nights/DATE-curate.json     the guide's rewrite and its usage
#   nights/DATE.md              the night, for reading
#   nights/DATE-positions.jsonl.xz   every pack a seat was handed, for the pick eval
#   stats.md, stats.json        every night's tables (ledger.py stats)
#   guide.md                    the guide the next night drafts with
#
# The run's own log, save and stdout stay under logs/draft-loop/DATE/
# (gitignored; DATE-<out> for a DRAFT_LOOP_OUT of its own). Seats are plan quota, never metered:
#   DRAFT_LOOP_MODEL    the drafting and debrief model (default claude-haiku-5-5)
#   DRAFT_LOOP_CURATOR  the guide's curator (default claude-sonnet-5-5)
#   DRAFT_LOOP_PLAYERS  seats (default 8); DRAFT_LOOP_BEST_OF (default 3)
#   MTG_DRAFT_RUNNER_BIN  a built runner (default: build the release one)
#   CLAUDE_CODE_BIN     a stand-in CLI, for a dry run
#   DRAFT_LOOP_OUT      where the kept files go (default reports/draft-loop)
set -eu
cd "$(dirname "$0")/../.."
night="${1:-$(date -u +%Y-%m-%d)}"   # a second run on one date: DATE_b
model="${DRAFT_LOOP_MODEL:-claude-haiku-5-5}"
players="${DRAFT_LOOP_PLAYERS:-8}"
best_of="${DRAFT_LOOP_BEST_OF:-3}"
# The date is the seed (a suffixed second run, DATE_b, a checksum of its name).
case "$night" in
  *_*) seed=$(printf '%s' "$night" | cksum | cut -d' ' -f1) ;;
  *) seed=$(printf '%s' "$night" | tr -d '-') ;;
esac
out="${DRAFT_LOOP_OUT:-reports/draft-loop}"
# The snapshot is the ledger's it was played for, and by the CLI it was
# played with: a stub dry run into a scratch DRAFT_LOOP_OUT kept its save
# where the real night of that date then carried it on, recording the
# stub's draft and tournament with no call made (#899).
logs="logs/draft-loop/$night"
if [ "$out" != "reports/draft-loop" ]; then
  logs="$logs-$(printf '%s' "$out" | tr -c 'A-Za-z0-9._-' '_')"
fi
run="out=$out cli=${CLAUDE_CODE_BIN:-claude} model=$model players=$players best_of=$best_of"
mkdir -p "$out/nights" "$logs"
if [ -f "$out/nights/$night.md" ]; then
  echo "$night is already in the ledger ($out/nights/$night.md); a second run that day is ${night}_b" >&2
  exit 1
fi
# A night that stopped partway (the container, a timeout) carries on from
# its snapshot when run again: the picks and matches already played stand.
resume=""
if [ -f "$logs/save.json" ]; then
  # A snapshot from before the stamp is carried on as it always was.
  if [ -f "$logs/run" ] && [ "$(cat "$logs/run")" != "$run" ]; then
    echo "$logs/save.json is another run's ($(cat "$logs/run"); this one is $run): move the directory away to start afresh" >&2
    exit 1
  fi
  resume="--resume $logs/save.json"
  echo "carrying on from $logs/save.json"
else
  # A fresh run starts with nothing of another's: the positions, the log
  # and the stdout are appended to, and the ledger reads all three.
  rm -f "$logs/positions.jsonl" "$logs/draft.log" "$logs/out.txt"
fi
printf '%s' "$run" > "$logs/run"
guide="$out/guide.md"
[ -f "$guide" ] || printf '# Innistrad draft guide\n\nNo nights yet: draft on what you know.\n' > "$guide"

if [ -z "${MTG_DRAFT_RUNNER_BIN:-}" ]; then
  cargo build --release -q -p mtg-draft-runner
  MTG_DRAFT_RUNNER_BIN=./target/release/mtg-draft-runner
fi

# The guide for the even seats, by seat flag; the runner records its hash.
guided=""
guide_flags=""
seat=0
while [ "$seat" -lt "$players" ]; do
  if [ $((seat % 2)) -eq 0 ]; then
    guided="$guided${guided:+,}$seat"
    guide_flags="$guide_flags --guide-$seat $guide"
  fi
  seat=$((seat + 1))
done

echo "draft loop $night: seed $seed, $players seats on cc:$model, best-of-$best_of, guided seats $guided"
# shellcheck disable=SC2086
MTG_DRAFT_POSITIONS="$PWD/$logs/positions.jsonl" "$MTG_DRAFT_RUNNER_BIN" \
  --model "cc:$model" --players "$players" --best-of "$best_of" --seed "$seed" --quiet \
  $guide_flags $resume --save "$logs/save.json" --log "$logs/draft.log" >> "$logs/out.txt" 2>&1 || {
    echo "the run failed; see $logs/out.txt" >&2; tail -20 "$logs/out.txt" >&2; exit 1; }

python3 scripts/draft-loop/ledger.py night --save "$logs/save.json" --positions "$logs/positions.jsonl" \
  --night "$night" --guided "$guided" --guide "$guide" --usage "$logs/out.txt" --log "$logs/draft.log" --out "$out"
xz -9 -c "$logs/positions.jsonl" > "$out/nights/$night-positions.jsonl.xz"
python3 scripts/draft-loop/reflect.py debrief --night "$night" --out "$out" --guide "$guide"
python3 scripts/draft-loop/ledger.py stats --out "$out"
python3 scripts/draft-loop/reflect.py curate --night "$night" --out "$out" --guide "$guide"
python3 scripts/draft-loop/report.py --night "$night" --out "$out"
echo "done: $out/nights/$night.md"
