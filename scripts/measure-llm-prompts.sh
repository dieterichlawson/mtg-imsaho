#!/bin/sh
# Count what an LLM seat is sent per game, for free, across several seeds.
#
# Runs `mtg-runner --p1 cc --p2 random` with `scripts/llm-prompt-stub.py`
# standing in for `claude`, then sums the stub's records: decisions per
# game, bytes of system prompt and decision prompt per call, and how many
# calls opened a session versus resumed one. Nothing is billed. The
# figures are bytes; divide by ~4 for a token estimate.
#
#   scripts/measure-llm-prompts.sh [seeds...]        # default: 1 2 3 4 5
#   MTG_LLM_HISTORY=2 scripts/measure-llm-prompts.sh  # with a history window
#   MTG_RUNNER_BIN=/elsewhere/mtg-runner scripts/measure-llm-prompts.sh
#                                                    # another build, e.g. before a change
#
# Output goes under logs/llm-prompts/<timestamp>/.
set -eu
cd "$(dirname "$0")/.."
seeds="${*:-1 2 3 4 5}"
# Absolute: the seat runs the CLI from a scratch directory of its own.
out="$PWD/logs/llm-prompts/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
if [ -z "${MTG_RUNNER_BIN:-}" ]; then
  cargo build --release -q
  MTG_RUNNER_BIN=./target/release/mtg-runner
fi
for seed in $seeds; do
  STUB_CALLS="$out/seed-$seed.calls" CLAUDE_CODE_BIN="$PWD/scripts/llm-prompt-stub.py" \
    "$MTG_RUNNER_BIN" --p1 cc --p2 random --seed "$seed" --quiet \
    --log "$out/seed-$seed.log" > "$out/seed-$seed.stdout" 2>&1 || true
done
python3 -I - "$out" $seeds <<'EOF'
import json, sys, statistics
out, seeds = sys.argv[1], sys.argv[2:]
print(f"{'seed':>5} {'calls':>6} {'opened':>7} {'resumed':>8} {'system B':>9} {'prompt B/call':>14} {'min':>5} {'max':>6} {'prompt B/game':>14} {'auto-pass':>9}")
totals = []
for seed in seeds:
    rows = [json.loads(l) for l in open(f"{out}/seed-{seed}.calls")]
    log = open(f"{out}/seed-{seed}.log", encoding="utf-8", errors="replace").read()
    auto = log.count("\tAUTO-PASS")
    if not rows:
        print(f"{seed:>5} {0:>6}"); continue
    prompts = [r["prompt_bytes"] for r in rows]
    opened = sum(r["session"] == "opened" for r in rows)
    resumed = sum(r["session"] == "resumed" for r in rows)
    print(f"{seed:>5} {len(rows):>6} {opened:>7} {resumed:>8} {rows[0]['system_bytes']:>9} "
          f"{statistics.mean(prompts):>14.0f} {min(prompts):>5} {max(prompts):>6} {sum(prompts):>14} {auto:>9}")
    totals.append((len(rows), rows[0]["system_bytes"], sum(prompts), auto))
if totals:
    n = len(totals)
    print(f"{'mean':>5} {sum(t[0] for t in totals)/n:>6.0f} {'':>7} {'':>8} {sum(t[1] for t in totals)/n:>9.0f} "
          f"{'':>14} {'':>5} {'':>6} {sum(t[2] for t in totals)/n:>14.0f} {sum(t[3] for t in totals)/n:>9.0f}")
EOF
echo "records under $out"
