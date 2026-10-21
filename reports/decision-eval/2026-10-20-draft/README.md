# Draft pick replay, 2026-10-20

The data behind `reports/llm-cost.md`, phase 4 (drafting).

- `positions.tar.xz` — every pick of two real two-seat drafts
  (`mtg-draft-runner --model cc:claude-sonnet-4-6 --players 2 --best-of 1`,
  seeds 21 and 22) as `MTG_DRAFT_POSITIONS` kept them on master at f898e3f:
  `base/positions-21.jsonl` and `base/positions-22.jsonl`, 84 picks each,
  and `positions-asked.jsonl`, the 156 of them with more than one card in
  the pack — the positions scored. Each carries the pack, the pool, the
  seat's place at the table, the notes it brought to the pick and the card
  it took live.
- `results.tar.xz` — one JSON line per position (and sample) per
  configuration, as `scripts/decision-eval/draft-replay.py` writes them:

  | file | seat | prompt |
  | --- | --- | --- |
  | `ref-opus-high` | `cc:claude-opus-5-5`, `MTG_LLM_THINKING=high`, 2 samples | f898e3f: rules and the set's card reference in the system prompt |
  | `base-sonnet` | `cc:claude-sonnet-4-6`, `off` | f898e3f |
  | `base-sonnet55` | `cc:claude-sonnet-5-5`, `off` | f898e3f |
  | `base-haiku55` | `cc:claude-haiku-5-5`, `off` | f898e3f |
  | `vara-sonnet` | `cc:claude-sonnet-4-6`, `off` | the variant below |
  | `vara-haiku55` | `cc:claude-haiku-5-5`, `off` | the variant below |
  | `ref-opus-high-vara` | `cc:claude-opus-5-5`, `high`, 1 sample | the variant below |
  | `h55-pick-schema` | `cc:claude-haiku-5-5`, `off`, 2 samples, `--json-schema` | af93e27 (`llm-cost.md`, phase 5) |
  | `h55-pick-text` | the same, the answer asked in the prompt (`MTG_CLAUDE_CODE_DRAFT_ANSWERS=text`) | af93e27 |

- `variant-pack-text.patch` — the variant that was measured and not
  taken: no card reference in the system prompt; each pack card's rules
  text under its line in the pick prompt, and the pool's in the deck-build
  prompt. It applies to 5e52396. It cut a pick's tokens about sevenfold and
  its price by a quarter at most, and Opus itself changed 18% of its
  settled picks without the reference (`reports/llm-cost.md`, phase 4).

To score another seat or build against the same reference without
spending the reference again:

```bash
mkdir -p logs/draft-eval/res && tar -C logs/draft-eval -xJf reports/decision-eval/2026-10-20-draft/positions.tar.xz
tar -C logs/draft-eval/res -xJf reports/decision-eval/2026-10-20-draft/results.tar.xz
cargo build --release -p mtg-draft-runner
scripts/decision-eval/draft-replay.py --positions logs/draft-eval/positions-asked.jsonl \
  --out logs/draft-eval/res --name mine --model cc:claude-haiku-5-5
scripts/decision-eval/compare.py --out logs/draft-eval/res --ref ref-opus-high base-haiku55 mine
```

A pick replays as the prompt it was live on any build that renders pick
prompts as f898e3f did (`mtg-draft-runner/tests/kept_picks_replay.rs`
holds the replay to that); a build that changes the pick prompt sends its
own prompt for the same pack and pool, which is the point of comparing
builds on it.
