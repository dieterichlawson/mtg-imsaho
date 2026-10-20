#!/bin/sh
# Count what a drafting seat is sent per draft, for free, across several seeds.
#
# The draft's counterpart of `scripts/measure-llm-prompts.sh`: runs
# `mtg-draft-runner --model cc --players 2 --best-of 1` with
# `scripts/llm-prompt-stub.py` standing in for `claude`, then sums the
# stub's records for the draft's own calls — the picks and the deck builds;
# the tournament's game calls are the other script's to count. Nothing is
# billed. The figures are bytes; divide by ~3 for a token estimate.
#
#   scripts/measure-draft-prompts.sh [seeds...]          # default: 1 2 3
#   MTG_DRAFT_RUNNER_BIN=/elsewhere/mtg-draft-runner scripts/measure-draft-prompts.sh
#                                                       # another build, e.g. before a change
#
# `picks` is the calls a draft asked a pick of: 39 a seat, a pack's last
# card being taken without asking. `system B` is the system prompt, which is
# the cached prefix: the rules and the set's card reference, read from the
# cache at a tenth of the input price after a seat's first call. `pick B`
# is the mean pick prompt (notes, pack, pool), which is not. `schemas` is
# how many different schemas the draft sent: each new one is a write of the
# whole system prompt, so it should be two, the pick's and the deck's.
#
# Output goes under logs/draft-prompts/<timestamp>/.
set -eu
cd "$(dirname "$0")/.."
seeds="${*:-1 2 3}"
out="$PWD/logs/draft-prompts/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
if [ -z "${MTG_DRAFT_RUNNER_BIN:-}" ]; then
  cargo build --release -q -p mtg-draft-runner
  MTG_DRAFT_RUNNER_BIN=./target/release/mtg-draft-runner
fi
for seed in $seeds; do
  STUB_CALLS="$out/seed-$seed.calls" CLAUDE_CODE_BIN="$PWD/scripts/llm-prompt-stub.py" \
    "$MTG_DRAFT_RUNNER_BIN" --model cc --players 2 --best-of 1 --seed "$seed" --quiet \
    --log "$out/seed-$seed.log" > "$out/seed-$seed.stdout" 2>&1 || true
done
python3 -I - "$out" $seeds <<'EOF'
import json, sys, statistics
out, seeds = sys.argv[1], sys.argv[2:]
print(f"{'seed':>5} {'picks':>6} {'decks':>6} {'system B':>9} {'pick B':>7} {'min':>5} {'max':>6} {'deck B':>7} {'schema B':>9} {'KB/draft':>9} {'schemas':>8}")
totals = []
for seed in seeds:
    rows = [json.loads(l) for l in open(f"{out}/seed-{seed}.calls")]
    props = lambda r: r.get("schema", {}).get("properties", {})
    picks = [r for r in rows if "pick" in props(r)]
    decks = [r for r in rows if "maindeck" in props(r)]
    draft = picks + decks
    if not picks:
        print(f"{seed:>5} {0:>6}"); continue
    pick_b = [r["prompt_bytes"] for r in picks]
    deck_b = statistics.mean(r["prompt_bytes"] for r in decks) if decks else 0
    system = statistics.mean(r["system_bytes"] for r in draft)
    schema = statistics.mean(r["schema_bytes"] for r in draft)
    sent = sum(r["system_bytes"] + r["prompt_bytes"] + r["schema_bytes"] for r in draft) / 1000
    kinds = len({r.get("schema_sha") for r in draft})
    print(f"{seed:>5} {len(picks):>6} {len(decks):>6} {system:>9.0f} {statistics.mean(pick_b):>7.0f} {min(pick_b):>5} {max(pick_b):>6} "
          f"{deck_b:>7.0f} {schema:>9.0f} {sent:>9.0f} {kinds:>8}")
    totals.append((len(picks), len(decks), system, statistics.mean(pick_b), deck_b, schema, sent, kinds))
if totals:
    n = len(totals)
    m = lambda i: sum(t[i] for t in totals) / n
    print(f"{'mean':>5} {m(0):>6.0f} {m(1):>6.0f} {m(2):>9.0f} {m(3):>7.0f} {'':>5} {'':>6} {m(4):>7.0f} {m(5):>9.0f} {m(6):>9.0f} {m(7):>8.0f}")
EOF
echo "records under $out"
