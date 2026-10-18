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

**Phase 3 (2026-10-18, its own section below)**: five real games each,
per turn, against phase 2's end on the same seat —

| | tokens / turn | $ / turn | games won |
| --- | ---: | ---: | ---: |
| phase 2's end, `claude-sonnet-4-6` | 23,183 | 0.1109 | 5 / 5 |
| phase 3, `claude-sonnet-4-6` | 7,614 (3.0x fewer) | 0.0243 (4.6x less) | 5 / 5 |
| phase 3, `claude-haiku-5-5`, now the default | 10,614 (2.2x fewer) | 0.0012 (92x less) | 5 / 5 |

On a replay of 104 real positions scored against a stronger model, the
phase-3 Sonnet chose as phase 2's did (79.8% against 80.8%) and the Haiku
chose better (87.5%). Most of the tokens left are the cached system
prompt, the cheapest a call sends; the bill moved more than the count.

Phases 1 and 2. Phase 1 (below) made a decision stateless and the
prompt around it smaller; phase 2 (the last section) lets the seat say
"go" so that fewer decisions are model calls at all, caps the output, and
asks for two sentences of thoughts. Same seeds, same decks, same seat
(`cc:claude-sonnet-4-6` against `random`), one game each:

| | before, seed 1 | before, seed 2 | phase 1, seed 1 | phase 1, seed 2 | phase 2, seed 1 | phase 2, seed 2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| model calls | 89 | 118 | 26 | 122 | 33 | 68 |
| priority offers passed unasked | 0 | 0 | 0 | 0 | 33 | 85 |
| final turn | 14 | 27 | 12 | 25 | 12 | 25 |
| output per call | ≈ 360 | ≈ 400 | 1,040 (728 thinking) | 672 (416 thinking) | 713 (506 thinking) | 670 (461 thinking) |
| $ per game (Sonnet 4.6) | 5.12 | 6.01 | 0.71 | 2.27 | 0.71 | 1.39 |
| $ per game (Haiku 4.5) | 1.71 | 2.00 | 0.24 | 0.76 | 0.24 | 0.46 |
| $ per call (Sonnet) | 0.058 | 0.051 | 0.027 | 0.019 | 0.022 | 0.020 |
| $ per call (Haiku) | 0.019 | 0.017 | 0.009 | 0.006 | 0.007 | 0.007 |
| cumulative, per game (Sonnet) | 1x | 1x | 7.2x | 2.6x | 7.2x | 4.3x |
| cumulative, per call (Sonnet) | 1x | 1x | 2.1x | 2.7x | 2.7x | 2.5x |

Read the per-game row with care: the games are not the same game (the
seat plays differently, so the counts differ), and the per-call row is
the one that holds across games. What phase 2 moved is the *number* of
calls for a given game: of the priority offers the seat would have been
asked in its two games, 50% (33 of 66) and 56% (85 of 153) were passed
without a call — a 2.0–2.3x cut in calls, short of the 3x aimed at, and
the phase-2 section says where the rest is. Per call, the two-sentence
thoughts and the output cap took the output from 1,040 to ≈ 700 tokens;
the thinking (≈ 480 tokens per call) is now two thirds of what a decision
costs.

### Phase 1 in detail

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

## Phase 2: the seat says "go"

Measured 2026-10-08/09, the same way as phase 1 (`mtg-runner --p1
cc:claude-sonnet-4-6 --p2 random`, seeds 1 and 2, `red-green` against
`white-black`, the `claude -p` seat's own `usage` numbers priced at the
rates in `cost.rs`). The two games ran at the same time as each other and
as the workspace test suite.

Phase 1 left a bill linear in the number of calls, and 90% of the calls
were a priority offer with something castable — mostly the seat holding a
Lightning Bolt and being asked at every step of both players' turns. A
person holding an instant says "go" and responds when something happens;
the page has it as the `f` key and the CLI as its auto-pass mode. Phase 2
gives the LLM seat the same thing as one row on every priority menu, right
after `Pass`: `Pass until something happens`. Picked, it passes now and
every later plain priority offer without a model call, until a conservative
stop — anything new on the stack, the seat's own turn beginning, attackers
declared against it, a block against its attacker, its next main phase (or
the same one with a land drop or a sorcery-speed cast on offer), a combat
prompt, any prompt that is not a plain priority pass — after which the
seat is asked with the full prompt, a line saying what stopped it, and the
recap of everything that happened meanwhile. `docs/llm-harness.md` has the
details; `AUTO_PASS` / `AUTO_PASS_STOP` are the log labels.

| | phase 1, seed 1 | phase 1, seed 2 | phase 2, seed 1 | phase 2, seed 2 |
| --- | ---: | ---: | ---: | ---: |
| model calls | 26 | 122 | 33 | 68 |
| pass-until taken / offers passed unasked | — | — | 16 / 33 | 35 / 85 |
| final turn | 12 | 25 | 12 | 25 |
| input (uncached) | 56 | 253 | 70 | 145 |
| cache read | 73,742 | 446,636 | 115,955 | 249,200 |
| cache write | 74,117 | 240,345 | 86,374 | 168,786 |
| output | 27,035 (18,934 thinking) | 82,035 (50,766 thinking) | 23,535 (16,704 thinking) | 45,591 (31,381 thinking) |
| output per call, mean / max | 1,040 / — | 672 / — | 713 / 2,000 | 670 / 2,769 |
| `thoughts`, chars per call | ≈ 297 | — | 213 | 219 |
| $ at Sonnet 4.6 rates | 0.71 | 2.27 | 0.71 | 1.39 |
| $ at Haiku 4.5 rates | 0.24 | 0.76 | 0.24 | 0.46 |
| $ / call (Sonnet) | 0.027 | 0.019 | 0.022 | 0.020 |
| $ / call (Haiku) | 0.009 | 0.006 | 0.007 | 0.007 |
| rejected / unanswered | 0 | 0 | 0 | 0 |

The seat won both games. What the table says:

- **Calls.** For a given game the right comparison is calls against the
  offers the seat would otherwise have been asked: 33 of 66 (seed 1) and
  85 of 153 (seed 2) were passed without a call, a 2.0–2.3x cut. Against
  phase 1's own games the per-game figures are noisier — seed 1 cost the
  same $0.71 with 33 calls where phase 1's game had 26, because that seat
  cast its Bolts early and was asked little; seed 2 went from 122 calls to
  68 and from $2.27 to $1.39 (1.6x).
- **Where the rest of the 3x is.** Of the 50 stops in the two games, 19
  were "your main phase" and 9 "your turn began": the conservative stops
  the design asks for, at which the seat mostly found nothing to do and
  said "go" again (a stop at its own upkeep is followed by a stop at its
  main phase one call later). Those are the cheap calls — 160–230 output
  tokens, little thinking — but they are calls. Stopping at a main phase
  only when there is a sorcery-speed play, and not at the upkeep at all,
  would take out up to 28 of the 101 calls here (fewer in practice: some
  of those main phases had a play to make); the stops for the opponent's
  spells (6), attacks (1), blocks (4) and combat prompts (9) are the ones
  a human would want and are unchanged by that.
- **Per call.** The two-sentence `thoughts` (213–219 characters against
  ≈ 300) and the `max_tokens` cap (4,096 at the default level, from
  8,192) took the output per call to ≈ 700 tokens; the thinking is
  ≈ 480 of those and is now two thirds of what a call costs at Sonnet
  rates. No call reached the cap (the largest answers were 2,000 and
  2,769 tokens, thinking included) and none was truncated or rejected.
- **Did the play get worse?** Not visibly. In seed 1 the seat held two
  Bolts to the end and finished the game with them at its turn-12 main
  phase 2; when the opponent's Walking Corpse blocked its Tusker it was
  asked (a block is a stop) and let the Tusker kill the Corpse rather
  than spend a Bolt. In seed 2 it was asked at every one of the
  opponent's spells (Swords to Plowshares, Doom Blade, Savannah Lions,
  Walking Corpse), and at the one attack against it — Savannah Lions
  into an empty board at 24 life — it declined to Bolt and to block,
  which is the ordinary play. No instant was held through an attack it
  should have answered: every attack against the seat is a stop, so there
  is no such window to miss. The row never answers a decision for the
  seat — a combat prompt, a target, a set, a sacrifice are always asked —
  and the one thing it does pass is the instant-speed windows in
  between, which the prompt tells the seat and which it can keep by
  answering `Pass`.

Free measurement (`scripts/measure-llm-prompts.sh`, the stub taking the
row with probability 0.5 or 0.8, `MTG_LLM_PASS_UNTIL=off` for "before"),
five seeds each, re-measured 2026-10-09 after two corrections (#754): the
script's "offers passed unasked" counted every `AUTO_PASS` line, which
labels the engagement as well as each unasked pass, so the column was
engagements plus unasked passes (seed 1's "57" was 44 + 13); and the stub's
schema fill picked the row on its own besides, so "half the time" was more
than half. Both are fixed; the figures below are with the row as it now
stands (its name only, the stops in the system prompt, #757):

| | calls / game | turns / game | calls / turn | engaged | offers passed unasked | prompt B / call | schema B / call | system B |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before (row off) | 99, 133, 78, 56, 109 (mean 95) | 86, 84, 54, 59, 76 (mean 72) | 1.32 | 0 | 0 | 881–1,350 | 333 | 12,647 |
| after, stub takes the row half the time | 108, 70, 67, 76, 143 (mean 93) | 98, 52, 56, 64, 108 (mean 76) | 1.22 | 33 | 7, 15, 20, 13, 43 (mean 20) | 1,092–1,485 | 385 | 13,934 |
| after, 80% of the time | 141, 77, 130, 101, 205 (mean 131) | 136, 92, 134, 94, 138 (mean 119) | 1.10 | 78 | 175, 21, 79, 95, 183 (mean 111) | 1,168–1,600 | 381 | 13,934 |

The stub is not the seat: a stub that passes instead of acting plays a
longer game, so its calls per game do not fall while its calls per turn
do, and a stub's answers are a hash of the prompt, so changing a prompt's
text replays a different game. The real games above are the measurement;
this is the shape check. Phase 2's real-game "taken / passed unasked"
figures were read off their logs by hand, not by the script; whether
they too counted engagements cannot be checked, as those logs were not
kept.

What the row costs per call, which 80e0018 did not measure (#757): as
first shipped it restated its stops in every priority prompt (≈ 300 B a
prompt, 13–17% of a game's prompt bytes), took the schema's mean from
≈ 335 B to ≈ 465 B, and added ≈ 1.6 KB to the system prompt whether the
row was offered or not (#755). It now names itself (`1: Pass until
something happens`); the one list of stops is a section of the system
prompt sent only to a seat that is offered the row; and the schema's
index description points there. Measured with the same stub, seeds 1–5,
row on: system prompt 14,229 → 13,934 B, schema 461 → 385 B per call;
row off: system prompt 14,229 → 12,647 B.

What else changed in phase 2:

1. `max_tokens` is a cap sized to the thinking level (`max_output_tokens`):
   2,048 for `off` (the largest structured answer — the draft runner's
   40-card deck — is ≈ 600), 4,096 for the default `low`, 8,192 and
   16,384 above. A `claude -p` seat gets the same number as
   `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, and writes a `USAGE` line per call so
   the largest answer of a game is read off the log rather than guessed.
2. Every schema's `thoughts` asks for at most two sentences, in the draft
   runner's two copies too.
3. `MTG_LLM_PASS_UNTIL=off` leaves the row out, for the A/B above. The
   draft runner's games use the same seat and got the row for free.

Phase-2 caveats, beyond phase 1's: one game per seed, and the two games
ran together with the test suite (wall time per call 15 s, as in phase
1). The stop at a block against the seat's own attacker is one the design
did not list and the implementation added on the "when in doubt, ask"
side; it fired four times in the two games.

## Phase 3: a turn in one answer, a cache that holds, and the model

Measured 2026-10-18. Asked for "the next 10x": the bill is now about 4.6x
lower per turn on the same model (about 3x on the Messages API, whose
baseline thought less), and about 92x lower on the model the harness now
defaults to (about 46x on the API), which played better on a
decision-quality replay. Raw tokens per turn fell about 3x on the same
model; most of what is left is the cached system prompt, the cheapest
token a call sends, so the count moved less than the bill.

### What was measured

- **Real games**, five per configuration, the same five each time:
  `--p1 cc:<model> --p2 random`, seeds 1 and 2 with the default decks
  (`red-green` against `white-black`), and three drafted-deck pairs
  (`gw-humans`/`rb-vampires` seed 11, `ub-zombies`/`ug-spider-spawning`
  seed 12, `rb-vampires`/`gw-humans` seed 13). The token figures are the
  `claude -p` seat's own `USAGE` lines; the dollars are those tokens at
  each model's API rates, cache writes at the CLI's 1-hour rate (2x
  input). The baseline is master at 4b6f3b7, phase 2's end.
- **Decision quality**: `scripts/decision-eval/`, its data kept in
  `reports/decision-eval/2026-10-18/`. Every model call of the
  three drafted-deck baseline games was kept as a save (113 positions);
  each configuration replays every position as one decision
  (`--resume --stop-after 1`) and is scored by how often its engine action
  matches a reference — a stronger model (`cc:claude-opus-5-5`) at
  `MTG_LLM_THINKING=high`, two samples, kept where both samples agree (104
  "settled" positions). One sample per configuration.
- **The Messages API path** could not be run (no key). Its figures below
  are modelled from the same measurements: the bytes each build sends for
  the same positions, 3.1 bytes a token (fitted from the cache-read sizes
  of games whose system prompts differ in length), the measured output,
  and the measured share of calls that miss the cache.

### Results

Real games, per turn (a game's turns, both players'):

| | calls / turn | tokens / turn | $ / turn | cold cache writes | thinking | games won |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline, `claude-sonnet-4-6` (4b6f3b7) | 2.61 | 23,183 | 0.1109 | 58 of 217 calls | 223,787 | 5 / 5 |
| final, `claude-sonnet-4-6` (ff4ed99) | 1.56 | 7,614 | 0.0243 | 18 of 150 | 0 | 5 / 5 |
| final, `claude-haiku-5-5` (ff4ed99) | 1.66 | 10,614 | 0.0012 | 17 of 146 | 43,933 | 5 / 5 |
| factor, Sonnet | 1.7x | **3.0x** | **4.6x** | | | |
| factor, Haiku | 1.6x | 2.2x | **92x** | | | |

The baseline thought at the CLI's own default effort (see "The baseline
overstated thinking" below). A cold write is a call that read no system
prompt from the cache: the first use of each kind of question in a game
— a mulligan, a main phase, a block — one each. Two earlier rounds on
near-final builds read the same way: the Sonnet at d1f4ba1, 1.35 calls, 6,241 tokens and $0.0186 a turn (its cold
writes, 7, were fewer because the decision replay ran beside it on the
same build and warmed the same cache), with one game lost by concession
(caveats); the Haiku at d465581, before the rules said which pass to
prefer, 2.11 calls, 13,190 tokens and $0.0015 a turn.

Decision quality, the 104 settled positions:

| configuration | agrees with the reference | output / decision | system prompt | prompt |
| --- | ---: | ---: | ---: | ---: |
| baseline, Sonnet, `low` (at the API seat's effort) | 80.8% (80/99) | 216 (84 thinking) | 17,920 B | 2,411 B |
| baseline, Sonnet, `off` | 78.8% (78/99) | 181 | 17,920 B | 2,411 B |
| final, Sonnet, `off` | 79.8% (83/104) | 190 | 9,096 B | 1,841 B |
| final, Haiku, `off` (effort `low`) | **87.5% (91/104)** | 546 (332 thinking) | 9,096 B | 1,841 B |
| final, Haiku, CLI default effort | 89.4% (93/104) | 757 (533 thinking) | 9,096 B | 1,841 B |

The final Sonnet chooses as the baseline did, within a position. The
Haiku chooses the reference's action more often than either Sonnet: 11
positions only it got and 3 only the final Sonnet got (exact McNemar
p = 0.06; 13 against 3, p = 0.02, at the CLI's default effort). The
reference is the Haiku's own generation, which may share its habits, so
read "agrees more often" as "is not worse", not as proof of the opposite.

The Messages API path, per decision and per turn, modelled as above:

| | system (cached) | prompt | output | cold writes | $ / decision | $ / turn | raw tokens / turn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline, Sonnet, `low` | 5,780 | 780 | 216 | 27% of calls | 0.0126 | 0.033 | 17,700 |
| final, Sonnet, `off` | 2,930 | 590 | 207 | 12% | 0.0070 | 0.0109 | 5,800 |
| final, Haiku, `off` | 3,810 | 770 | 523 | 12% | 0.0004 | 0.0007 | 8,500 |

(Token counts per decision; output as the final real games measured it,
and the baseline's as the replay measured it at the effort the API seat
sends, its games having thought at the CLI's default; calls per turn as
the real games made them; Haiku's tokenizer counts the same
text as about 30% more tokens.) On the API path the same-model factor is
about 3x per turn, in dollars and in raw tokens, and the Haiku's about
46x in dollars; the CLI path's larger same-model factor is the thinking
its baseline did at the CLI's default effort.

### What changed, and what each moved

1. **The cache holds** (the largest single term). The answer schema is
   part of the cached prefix on both paths — a changed
   `output_config.format` invalidates the Messages API's prompt cache, and
   `claude -p` sends the schema as a tool ahead of the system prompt — and
   every menu's `enum: [0..n]`, the plan's card names and the counts in
   descriptions made nearly every schema new. Measured on the first build
   of this phase: 60 to 95% of a game's calls read nothing from the cache
   and wrote the whole prefix again at 1.25-2x where a read is 0.1x. Now
   the schema goes without its per-call values (`cache_stable_schema`),
   every index question is one schema, the answer is checked against the
   full schema (one `REASK` with the reason when it breaks it, and a menu
   row copied into a plan read as its card), and
   `mtg-runner/tests/schema_cache.rs` holds every kind of question to one
   sent schema. Cold writes: 58 of 217 calls at the baseline, 18 of 150 —
   one per kind of question a game asks.
   The draft's pick schema got the same treatment; there every pick's
   schema was new, and the system prompt it rewrote was the set's card
   reference.
2. **Fewer calls.** The seat is not asked to respond to its own spell
   while only its own are on the stack; the pass-until row no longer
   stops at the seat's own upkeep, and stops for the opponent's attack
   once blocks are in. A main-phase answer carries `then` (cards to play
   after `action`), `ask_again` (false, the usual answer, ends the main
   phase: an empty plan used to mean "ask me again", and a seat with
   nothing more to do was asked again to say so) and `attack` (declared
   at combat without a call if nothing happened in between). Calls per
   turn: 2.61 to 1.56.
3. **No thinking by default.** The reasoning goes in the answer's
   `thoughts` on every backend (the Messages API seat used to strip it and
   read a thinking block that is empty at `off` and on the newest models).
   On the replay `off` chose as well as `low`. The `claude -p` seat now
   passes `--effort` for its level; without it the CLI ran its own default
   effort and thought 476-5,212 tokens a decision where `--effort low`
   thought 25-71.
4. **A smaller system prompt**: 17,920 to 9,096 bytes for a drafted deck
   (12,647 to 6,409 for a preset one). The rules restate only what the
   prompt does not show; a decklist entry says each thing once, without
   reminder text.
5. **A smaller prompt**: 2,411 to 1,841 bytes on the same positions. Mana
   taps and pools emptying are not events; an opponent's card in a
   graveyard is described only where its text works from there (the text
   of every dead creature had been a third of a late-game prompt);
   repeated events and stack entries are one line with `(×N)`; graveyards
   are grouped by name.
6. **The `claude -p` child is not part of the session that started it.**
   Run from a cloud session, it inherited that session's variables, so
   every decision also asked a second model for a session title (≈ 1,200
   input and 200 output tokens, in no usage line the seat reads) and wrote
   the session's reminders into every prompt (1,388 tokens of cache write
   a call, 798 without them).
7. **The model.** `claude` with no model now runs `claude-haiku-5-5`
   (`DEFAULT_ANTHROPIC_MODEL`); the evidence is the two tables above. Name
   a model to run another. The current Haiku is asked for adaptive thinking
   (it was being sent a `budget_tokens`, a 400 on that model), `off`
   leaves it the output room `low` has, and its price and the current
   Opus's are in `cost.rs`.

### The baseline overstated thinking

Phases 1 and 2 measured the `claude -p` seat, which then passed no
`--effort`: at `MTG_LLM_THINKING=low` the CLI ran its own default effort,
and on the same positions it thought 476-5,212 tokens a decision where
`--effort low` — what the Messages API seat sends at `low` — thought
25-71. The CLI-path tables of those phases are what the CLI seat cost;
the API seat at `low` always thought far less. The API-path table above
models the baseline at `low` as the API seat would have run it.

### Caveats

- One game per seed and configuration, against `random`. The games are
  not the same game; per turn is the comparison that holds.
- Two of the Sonnet's games on earlier builds this phase ended in a
  concession the board did not call for (36 "trample" damage where one
  attacker had trample; a life total its own mill land could have
  outrun). The baseline never chose Concede. The rules now say when to
  concede and the confirmation asks the seat to count it out first; the
  final round conceded nothing, and whether that holds is a playtest
  question (H31's kind). The Haiku conceded none of its ten.
- The Haiku answered plain `Pass` on the opponent's turn where the
  Sonnet said "go", and made 2.11 calls a turn to the Sonnet's 1.35; with
  the rules saying the row is the usual way to pass, 1.66 to 1.56.
- The draft runner's picks were not re-measured in real drafts; the
  pick-schema change is pinned by its tests.
- The Messages API figures are a model of measurements, not a bill.

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
MTG_LLM_PASS_UNTIL=off scripts/measure-llm-prompts.sh 1 2 3 4 5   # without the pass-until row
STUB_PASS_UNTIL=0.8 scripts/measure-llm-prompts.sh 1 2 3 4 5      # the stub takes it 80% of the time

# Phase 2's real games, the same way:
./target/release/mtg-runner --p1 cc:claude-sonnet-4-6 --p2 random --seed 1 --quiet \
  --log logs/cost/phase2-seed1.log      # AUTO_PASS / AUTO_PASS_STOP / USAGE lines inside

# What shapes of prompt a run asked:
scripts/llm-prompt-shapes.py logs/cost/after-seed1.log

# Phase 3's games: the five, per model; read `USAGE` (cache_read 0 is a cold write).
for s in 1 2; do ./target/release/mtg-runner --p1 cc:claude-haiku-5-5 --p2 random --seed $s --quiet \
  --log logs/cost3/haiku/seed$s.log; done
./target/release/mtg-runner --p1 cc:claude-haiku-5-5 --p2 random --deck1 decks/gw-humans.txt \
  --deck2 decks/rb-vampires.txt --seed 11 --quiet --log logs/cost3/haiku/g1.log   # and 12, 13

# Phase 3's decision-quality replay (docs/llm-harness.md, "Saving and resuming"):
scripts/decision-eval/replay.py --positions logs/eval/positions --out logs/eval/res \
  --name haiku --p1 cc:claude-haiku-5-5
scripts/decision-eval/compare.py --out logs/eval/res --ref ref-opus-high haiku
```
