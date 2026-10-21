# Decision-quality replay, 2026-10-18

The data behind `reports/llm-cost.md`, phase 3.

- `positions.tar.xz` — 113 saves, one per model call of three real games
  (`cc:claude-sonnet-4-6` against `random`, the drafted-deck pairs
  `gw-humans`/`rb-vampires` seed 11, `ub-zombies`/`ug-spider-spawning`
  seed 12, `rb-vampires`/`gw-humans` seed 13), kept by
  `scripts/decision-eval/capture-claude.py` on master at 4b6f3b7.
- `results.tar.xz` — one JSON line per position (and sample) per
  configuration, as `scripts/decision-eval/replay.py` writes them:

  | file | seat | build |
  | --- | --- | --- |
  | `ref-opus-high` | `cc:claude-opus-5-5`, `MTG_LLM_THINKING=high`, 2 samples | 4b6f3b7 |
  | `base-low-eff` | `cc:claude-sonnet-4-6`, `low`, `--effort low` by wrapper | 4b6f3b7 |
  | `base-off-eff` | `cc:claude-sonnet-4-6`, `off`, `--effort low` by wrapper | 4b6f3b7 |
  | `base-low-clidefault` | `cc:claude-sonnet-4-6`, `low`, the CLI's own effort (partial) | 4b6f3b7 |
  | `new3-off` | `cc:claude-sonnet-4-6`, `off` | 52eac98 |
  | `new4-off` | `cc:claude-sonnet-4-6`, `off` | d1f4ba1 |
  | `new4-haiku55-off` | `cc:claude-haiku-5-5`, `off`, the CLI's own effort | d1f4ba1 |
  | `new5-haiku55-off` | `cc:claude-haiku-5-5`, `off`, `--effort low` | 55b9254 |
  | `h55-schema` | `cc:claude-haiku-5-5`, `off`, 2 samples, `MTG_CLAUDE_CODE_ANSWERS=schema` | 121cd89 (`llm-cost.md`, phase 5) |
  | `h55-text` | the same, the answer asked in the prompt (the default since) | 121cd89 |

To score a new configuration against the same reference without spending
the reference again:

```bash
mkdir -p logs/eval/res && tar -C logs/eval -xJf reports/decision-eval/2026-10-18/positions.tar.xz
tar -C logs/eval/res -xJf reports/decision-eval/2026-10-18/results.tar.xz
scripts/decision-eval/replay.py --positions logs/eval/positions --out logs/eval/res \
  --name mine --p1 cc:claude-haiku-5-5
scripts/decision-eval/compare.py --out logs/eval/res --ref ref-opus-high new5-haiku55-off mine
```

A position replays as the prompt it was live only on a build that renders
prompts as 4b6f3b7 did; a later build sends its own prompt for the same
position, which is the point of comparing builds on it.
