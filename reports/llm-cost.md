# What an LLM-seat game costs, before and after

Measured 2026-10-08 on `mtg-runner --p1 cc:claude-sonnet-4-6 --p2 random`
(the `claude -p` seat, which records the same `usage` numbers the Messages
API returns), seeds 1 and 2, the default `red-green` deck. No
`ANTHROPIC_API_KEY` was available, so the metered seat's request path was
exercised only by its tests (a loopback server records the request
bodies); the dollar figures below are the recorded token counts priced at
the rates in `mtg-player/src/llm/cost.rs` (Sonnet 4.6: $3 / $15 per MTok
in / out, cache read $0.30, cache write $3.75 — the 5-minute-TTL write
rate; the CLI uses the 1-hour TTL, whose write rate is $6, so the "before"
column is if anything understated).

## Headline

| | before, seed 1 | before, seed 2 | after, seed 1 | after, seed 2 | after, seed 1, thinking off |
| --- | ---: | ---: | ---: | ---: | ---: |
| decisions (calls) | 89 | 118 | 26 | 122 | 51 |
| final turn | 14 | 27 | 12 | 25 | 14 |
| input (uncached) | 183 | 245 | 56 | 253 | 107 |
| cache read | 3,227,808 | 6,061,606 | 73,742 | 446,636 | 163,071 |
| cache write | 979,099 | 931,865 | 74,117 | 240,345 | 128,242 |
| output | 32,044 | 46,694 | 27,035 (18,934 thinking) | 82,035 (50,766 thinking) | 19,030 (0 thinking) |
| $ at Sonnet 4.6 rates | 5.12 | 6.01 | 0.71 | 2.27 | 0.82 |
| $ at Haiku 4.5 rates | 1.71 | 2.00 | 0.24 | 0.76 | 0.27 |
| $ / decision (Sonnet) | 0.058 | 0.051 | 0.027 | 0.019 | 0.016 |
| $ / decision (Haiku) | 0.019 | 0.017 | 0.009 | 0.006 | 0.005 |

The seat (p0, red-green) won every game. The games are not the same game:
the seat plays differently, so the decision counts differ (seed 1 ended on
turn 12 with 26 decisions after, turn 14 with 89 before — the "before"
seat passed 84% of its priority offers while holding a castable Lightning
Bolt and was therefore asked at every step; the "after" seat cast them).
The per-decision row is the comparison that holds across games.

**Per game, at Sonnet rates: 7x (seed 1), 2.6x (seed 2). Per
decision: 2.1–2.7x with the default thinking, 3.6x with thinking off.** The
100x the exercise aimed at is not reached, and the per-term table says
why: the two terms that made the bill quadratic are gone, and what is
left is the model's own output and the CLI's own cache writes, both
linear in the decision count and neither removable by a prompt change.

## Where the tokens went, per term

The "before" seat sent, on every decision, its whole conversation so far —
every earlier prompt and answer — in front of the new prompt, through a
`claude -p` session `--resume`d for the whole game. Each prompt restates
the whole position (board, hand, stack, the events since the last prompt,
every legal action), so the history was the same information again, and
the input tokens of a game grew with the square of its decision count.

| term, per decision | before | after | factor |
| --- | ---: | ---: | ---: |
| history re-read (cached) | ≈ 350 tok × decisions so far; 36k and 51k cache-read per call on average over the two games, 3.2M–6.1M per game | 0 | gone |
| cache written per call | 11.0k / 7.9k tok (the appended tail, re-cached at 1.25x) | 2.85k / 2.0k tok (2.5k with thinking off) | 4x |
| system prompt | 32,364 B ≈ 8.1k tok, in the cached prefix | 12,358 B ≈ 3.1k tok | 2.6x |
| decision prompt | ≈ 1.0 KB ≈ 250–300 tok | ≈ 1.4 KB ≈ 350 tok, the seat's notes (≤ 600 chars) at the top | ≈ 0.8x |
| output: answer + reasoning | ≈ 360–400 tok, thoughts ≈ 150 chars | 1,040 tok (728 thinking + ≈ 300-char thoughts); 373 tok with thinking off | 0.35x / 1x |
| $ of input terms (Sonnet) | 0.052 | 0.0115 | 4.5x |
| $ of output (Sonnet) | 0.0054 | 0.0156 / 0.0056 | 0.35x / 1x |

What this says:

- The quadratic term is gone. "Cache read" fell from 3.2–6.1M tokens per
  game to 74k–447k, and every call is now the same size as the first.
- The CLI writes ≈ 2.5–2.85k cache tokens on every call even with an
  identical system prompt, and at 1.25x the input rate that is now the
  largest *input* term ($0.009–0.011 per decision). That is the CLI's own
  cache layout — the Messages API seat puts one `cache_control` breakpoint
  on its system prompt and would read it (3.1k tokens, $0.0009) and write
  only the prompt; its input per decision is ≈ $0.002, 26x below the
  "before" input term. That seat could not be run here (no key); its
  request bodies are asserted by tests.
- The output term is what is left, and it got *bigger* with the default
  thinking: a seat that starts each decision from a fresh context reasons
  through the board each time (728 thinking tokens per decision, where the
  "before" seat, deep in a resumed session, spent ≈ 360 output tokens all
  in). With `MTG_LLM_THINKING=off` the output is back to 373 tokens per
  decision and the JSON `thoughts` are shorter, not longer (172 chars
  against 297) — the opposite of what a one-line probe suggested — and
  that game was also won. The default stays `low` (the CLI's own default),
  because thinking is the thing most likely to be worth paying for; the
  knob is there, and this is the number it moves.
- Haiku 4.5 rates divide everything by ≈ 3 without changing a prompt; the
  default model is still `claude-sonnet-4-6`.

## Decisions that reach the model

`scripts/llm-prompt-shapes.py` over the real logs and ten stub logs: no
prompt in any of them is a priority offer whose only rows are Pass,
Concede and mana taps. The engine already passes those itself
(`mtg-engine/src/engine/cards_flow.rs`, the auto-pass check), and the
seat's own `should_auto_pass` covers what gets through, so there was no
call to save there: 0 of 89 and 0 of 118 "before" prompts were of that
shape. What the seat is asked, by shape (before, seed 1 / seed 2):

| shape | seed 1 | seed 2 |
| --- | ---: | ---: |
| priority with something to cast or activate | 82 (92%) | 105 (89%) |
| declare attackers | 3 | 7 |
| declare blockers | 1 | 1 |
| target or set | 2 | 4 |
| mulligan | 1 | 1 |

## Free measurement, five seeds each

`scripts/measure-llm-prompts.sh` with a stub `claude` (nothing billed),
`MTG_RUNNER_BIN` pointed at the unmodified build for "before":

| | calls / game | system prompt | prompt bytes / call | opened / resumed sessions |
| --- | ---: | ---: | ---: | ---: |
| before | 102, 126, 104, 83, 138 (mean 111) | 32,364 B | 782–1,227 (mean ≈ 1,050) | 1 / all the rest |
| after | 99, 133, 78, 56, 109 (mean 95) | 12,358 B | 881–1,350 (mean ≈ 1,100) | every call / 0 |

(The stub answers at random from the schema, so its games are not the
real seat's games; the counts show the shape, not the play.)

## What changed

1. **A decision is sent without the history** (`MTG_LLM_HISTORY`, default
   0). The Messages API seat keeps a sliding window of that many
   exchanges; the Gemini and `claude -p` seats start a fresh conversation
   every window of decisions. Tests assert the bound on the request bodies
   themselves (decision six is no larger than decision one by more than the
   window) and that a `claude -p` call never `--resume`s an earlier
   decision's session unless a window asks for it.
2. **The seat's notes.** The tail (≤ 600 chars) of its reasoning from the
   last decision comes back at the top of the next prompt as `Your notes
   from your last decision:`, documented in the prompt format. That is how
   a plan ("hold Bolt for the flier") survives without the history.
3. **A resume's recap rides in the first prompt after it**, once, instead
   of being a permanent history entry; the `claude -p` seat no longer
   spends a call being told "ready".
4. **The system prompt says what decides a game.** `GAME_RULES` went from
   24 KB to 10.6 KB: the prompt-format contract, the engine's conventions
   (auto-tap, X funding, sacrifice and exile costs, the structured
   `order` / `amount` / `indices` prompts, CR 616.1), the mulligan, a few
   lines of play advice. Gone: a keyword glossary, five worked examples, a
   combat-math essay. Every string the contract tests pin is kept verbatim.
   The two response preambles are a third of their length.
5. **A thinking level** (`MTG_LLM_THINKING=off|low|medium|high|N`, default
   low): adaptive thinking plus `output_config.effort` on the current
   Anthropic models, `budget_tokens` on the older ones, no thinking
   parameter for `off`, `MAX_THINKING_TOKENS` on a `claude -p` child (left
   at the CLI's default for `low`: a probe spent 736 thinking tokens at the
   default and 1,455 with `MAX_THINKING_TOKENS=1024`, so a small named
   budget is not a way down on that path; `0` is).
6. **The draft runner's own copy** of the three backends (the one that has
   missed fixes before, #404) got the same window, the same thinking
   level, notes carried from pick to pick (`Your notes from your last
   pick:`), and a bug fix: with thinking on, the Messages API's first
   content block is the thinking block, and the draft read
   `content[0].text` — nothing — and retried a good answer as "empty
   text". The draft's *game* phase no longer puts the whole set's card
   reference (public in a draft, but ≈ 10k tokens re-read on every
   decision) in the game system prompt: every card that comes into view is
   described in the decision prompt itself.
7. **`mtg-runner` prints the cost**: dollars for a metered seat, and for a
   `claude-code:<model>` seat `n/a (plan quota; $X at API rates)`, with the
   thinking share of the output where the CLI reports it.

## What was deliberately kept

- Everything a human player has: the full board, hand, stack, graveyards,
  exile counts, the delta of events since the last prompt (capped at 80,
  with a marker), every legal action, and the rules text of every opponent
  card in view, re-sent on every prompt while it is in view. The prompt did
  not get smaller; the things around it did.
- The seat's own decklist with full rules text, in the system prompt.
- `should_auto_pass` as it was: Pass / Concede / mana-ability-only offers,
  nothing else. Holding priority to respond is a decision and is asked.
- The default model, `claude-sonnet-4-6`, and the default thinking level.
- The `THOUGHT` record on every decision, and the `SESSION` record — now one
  per decision for a `claude -p` seat, since each decision is a session.

## Caveats

- The after-change games ran while the test suite and the stub measurement
  were running on the same machine; per-decision wall time (PROMPT to
  RESPONSE) was 9–10 s before and 15–20 s after, and how much of that is a
  fresh CLI session per call versus load was not separated.
- A `claude -p` session per decision leaves one transcript per decision in
  the CLI's own project directory rather than one per game.
- The metered (`claude:` / `gemini:`) request paths were not exercised
  against a live API for lack of keys; their request shape is asserted by
  tests against a loopback server and the claude-api reference. The
  26x input figure for the API seat is computed from that shape, not
  measured.
- One game per configuration is one game; the per-decision figures are
  the stable ones, and even those move with how much the seat has to think
  about.

## How to re-measure

```bash
# Real, on plan quota (the CLI's default model refuses the game prompt; name one):
./target/release/mtg-runner --p1 cc:claude-sonnet-4-6 --p2 random --seed 1 --quiet \
  --log logs/cost/after-seed1.log      # TOKEN_USAGE and the cost line at the end
MTG_LLM_THINKING=off ./target/release/mtg-runner --p1 cc:claude-sonnet-4-6 --p2 random --seed 1 --quiet \
  --log logs/cost/after-seed1-nothink.log

# Free, across seeds, with a stub CLI:
scripts/measure-llm-prompts.sh 1 2 3 4 5
MTG_RUNNER_BIN=/path/to/old/mtg-runner scripts/measure-llm-prompts.sh 1 2 3 4 5

# What shapes of prompt a run asked:
scripts/llm-prompt-shapes.py logs/cost/after-seed1.log
```
