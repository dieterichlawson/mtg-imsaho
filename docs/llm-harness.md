# Running the LLM playtest harness

Two binaries drive LLM seats:

- `mtg-runner` — one game, two seats, fixed decks.
- `mtg-draft-runner` — an eight-seat draft of a set, then a Swiss tournament between the drafted decks.

Run both from the repo root: deck paths, `data/sets/`, and log paths are all
resolved relative to the working directory.

## Who pays for a seat

This is the distinction to get right before typing anything else.

| Seat spec | Backend | Needs first | Billed to |
| --- | --- | --- | --- |
| `cli` | you, at the keyboard | an interactive terminal | nothing |
| `random` | uniform random legal action | nothing | nothing |
| `claude[:model]` (aliases `ai`, `llm`) | Anthropic Messages API | `ANTHROPIC_API_KEY` | **a metered API bill, per token** |
| `gemini[:model]` | Gemini Interactions API | `GEMINI_API_KEY` | **a metered API bill, per token** |
| `claude-code[:model]` (alias `cc`) | `claude -p` subprocess | the `claude` binary on `PATH` | **the plan quota of whatever that CLI is logged into — no API bill** |

`claude` and `claude-code` speak the same prompt protocol to the same vendor;
the only difference is who gets charged. That is the whole reason the
`claude-code` seat exists. Use it for anything long-running, and reach for
`claude`/`gemini` only when a run genuinely needs a specific API model.

Both binaries refuse an unusable seat before the game starts rather than
failing on the first decision:

```
$ mtg-runner --p1 claude --p2 random
Error: --p1 claude needs an Anthropic API key: ANTHROPIC_API_KEY is not set; use --p1 claude-code to run the same seat through the Claude Code CLI, or --p1 random

$ mtg-runner --p1 cc --p2 random
Error: --p1 cc needs the Claude Code CLI: `claude` is not runnable (set CLAUDE_CODE_BIN to its path)
```

`CLAUDE_CODE_BIN` names the binary to run for a `claude-code` seat; without it
the seat runs `claude` from `PATH`. Setting it to a stub script is how the
tests exercise the seat without spending anything.

A `claude-code` seat runs the CLI *without* `ANTHROPIC_API_KEY` or
`ANTHROPIC_AUTH_TOKEN`, even when the shell has them exported for a `claude`
seat. The CLI gives such a key precedence over its claude.ai login, which
would quietly turn the seat into a metered one — and when the key cannot serve
the request, into one that fails every decision with `claude.ai connectors are
disabled because ANTHROPIC_API_KEY or another auth source is set`. A CLI that
has no login and can only run on a key gets it back with
`MTG_CLAUDE_CODE_INHERIT_AUTH=1`; that seat is then billed to the key.

Nor does it inherit the variables of a Claude Code session that started the
run (`CLAUDE_*` and `CCR_*`, but for the CLI's own provider, login, config
directory and client-certificate settings). Run from a cloud session, the
child otherwise called itself that session: each decision also asked a
second model for a session title, in no usage line the seat reads, and
carried the session's reminders into every prompt.
`MTG_CLAUDE_CODE_INHERIT_SESSION_ENV=1` keeps them.

Default models when the spec names none: `claude` → `claude-haiku-5-5`
(`DEFAULT_ANTHROPIC_MODEL`: replayed over real positions it chose the
reference's action more often than `claude-sonnet-4-6`, the default before
it, at about a twelfth of the price; `reports/llm-cost.md`, phase 3 — and
in the draft, replayed over 156 real picks, it took the reference's card on
77% of them to the older Sonnet's 58%, at a twentieth of the price; phase 4),
`gemini` → `gemini-2.5-flash`, `claude-code` → whatever the CLI defaults to
(no `--model` is passed). The CLI's default model has refused the game
prompt before; `cc:claude-haiku-5-5` is the same choice on plan quota.

## What one decision sends

Every decision is sent on its own. A call carries:

- the **system prompt** — the prompt-format contract and the engine's
  conventions (`GAME_RULES`, ~11KB), the match section, and the seat's own
  decklist with rules text. It is the same bytes on every call of a game, so
  the API's prompt cache serves it;
- **no earlier exchanges**, by default. Every decision prompt restates the
  whole position (board, hand, stack, the events since the last prompt, every
  legal action), so the history was the same information again, and re-sending
  it made a game's input tokens grow with the square of its decision count.
  `MTG_LLM_HISTORY=K` sends the last K exchanges too: a sliding window on the
  Messages API seat; a conversation restarted every K decisions on the Gemini
  and `claude -p` seats, which cannot slide a server-held one;
- the **decision prompt**, which opens with `Your notes from your last
  decision:` — the seat's own `thoughts` from the previous call (two
  sentences, at most 300 characters), which is how a plan carries across
  decisions without the history. A save carries the notes with the rest of
  what a seat remembers (below), so a resumed seat's next prompt is the one
  it would have had. Late in a game the prompt says each thing once: an
  opponent's card in a graveyard is described only where its text works from
  there (flashback; cast or returned from a graveyard), identical events and
  stack entries in a row are one line ending `(×N)`, a graveyard's cards are
  grouped by name, and mana taps and pools emptying are not events;
- the **answer schema**, which is part of the cached prefix too: on the
  Messages API a changed `output_config.format` invalidates the prompt cache,
  and through `claude -p` the schema is a tool definition ahead of the system
  prompt. So it goes without its per-call values (`cache_stable_schema`: no
  enums, no item counts, no count or index in a description) and is the same
  bytes every time a kind of question is asked; the answer is checked against
  the full schema instead, and one that breaks it is asked once more with the
  reason (`REASK` in the log). `mtg-runner/tests/schema_cache.rs` plays stub
  games and holds every kind of question to one schema.

A `claude -p` game seat is not sent the schema at all, by default: the
question's fields end the decision prompt (`Answer with only this JSON
object: {"thoughts": string, "action": integer 0 to 7}` and a line for each
field whose meaning is the question's own), the fields every game asks the
same way (`thoughts`, `then`, `ask_again`, `attack`) are explained once in
the system prompt's "Answer format", and the object is read out of the
result text. Through `--json-schema` the CLI made each answer a tool call and
a second request after it — about 575 tokens of cache write and 100 of
output a decision, whatever was asked — and each kind of question a new
tool ahead of the system prompt. Replayed on the kept positions it chose as
well as the schema did, for a fifth less (`reports/llm-cost.md`, phase 5).
A field the question does not read (the plan fields outside the main
phase) is left out of the line, so the answer does not spend output on it.
`MTG_CLAUDE_CODE_ANSWERS=schema` sends `--json-schema` as before. The draft
seat's picks and deck builds stay on `--json-schema`, where text answers
saved 6% and agreed with the reference less often;
`MTG_CLAUDE_CODE_DRAFT_ANSWERS=text` asks them in text.

Every `claude -p` child is run with `FORCE_PROMPT_CACHING_5M=1`: the CLI
otherwise writes its cache at the 1-hour TTL, 2x the input rate where the
5-minute TTL is 1.25x, and half of what a decision writes is its own prompt,
which no later call reads. A seat reads its system prompt back within
seconds. `MTG_CLAUDE_CODE_CACHE_TTL=1h` keeps the CLI's hour. One kind of
call keeps it anyway: a draft seat's sideboarding, which comes a game after
the seat's last call, writes at the hour, so the seat's next one reads its
prefix (the rules and the set's card reference) instead of writing it whole
again. The summary's API-rate figure prices writes at the 5-minute rate.

Every backend asks for the reasoning in the answer's `thoughts` field, one
or two sentences; it is the notes the next prompt opens with. Thinking is
billed as output, and `MTG_LLM_THINKING` sets it for every seat: `off` (the
default), `low`, `medium`, `high`, or a token budget such as `2048`. On the
current Anthropic models that is adaptive thinking plus `output_config.effort`
(the models that cannot turn thinking off get effort `low` at `off`); on the
older ones a `budget_tokens`; on a `claude -p` seat the CLI's own
`MAX_THINKING_TOKENS` and `--effort`, which the seat always passes — without
it the CLI ran its own default effort and thought ten to a hundred times as
much as the API seat it stands in for. A replay of real positions found
`off` choosing as well as `low` (`reports/llm-cost.md`, phase 3). Gemini
keeps its own `thinking_level`.

A `claude -p` seat therefore opens a session per decision (one `SESSION`
record each), and the draft seats do the same per pick. `reports/llm-cost.md`
has the measured before/after, and `scripts/measure-llm-prompts.sh` counts
decisions and prompt bytes per game for free with a stub CLI.

`max_tokens` is a cap on a runaway answer, not a budget: 4,096 at `off` and
`low` (the largest structured answer, the draft runner's 40-card deck, is
≈ 600, but the newest models think at `off` anyway, up to ≈ 1,400 tokens a
decision), 8,192 and 16,384 above, a budget plus the answer on the older
models. A `claude -p` seat gets the
same number as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`. Every schema's `thoughts`
field asks for at most two sentences.

## Saying "go": the pass-until row

A person holding an instant does not deliberate at every priority of both
players' turns; they say "go" and respond when something happens. The
browser page has this as the `f` key and the CLI as its auto-pass mode. An
LLM seat has it as one row on every priority menu, right after `Pass`:

```
0: Pass
1: Pass until something happens
2: Cast Lightning Bolt (tap Mountain)
3: Concede
```

Picking it passes now and keeps passing every later *plain* priority offer
without a model call, until something the seat would want to see happens.
The stops are where a decision may be owed, and not where none is:

- anything on the stack that was not there when it said "go" (an entry
  that was there resolving off the top is expected), or anything put on
  it since, however like an entry that was there it looks — a second
  activation of the same ability is new (the view's `stack_puts`, #750).
  The engine passes for a seat that has nothing to respond with, so an
  opponent's instant can come and go unseen; that is not a stop (no window
  is left, and the board shows it), but it does stop a plan and a planned
  attack — the view's `opponent_stack_puts` counts it (#861);
- the opponent attacking it, once blocks are in — the window a trick or a
  removal spell is cast in (the block itself is a combat prompt, asked);
- a blocker declared against its attacker;
- its own first main phase, and a later main phase of its own when a land
  drop or a sorcery-speed cast or activation is on offer — not the main
  phase it said it was done with;
- a target or set or resolution prompt, once the opponent has put
  something on the stack since the pass began. The prompt is always
  asked; one the seat's own cast or trigger raises (its X, its target, a
  "may pay") does not end the pass, nor a plan or the attack named with it
  (#862). A combat declaration is asked too, but the pass carries on past
  it, and declaring attackers starts one.

Its own turn beginning used to stop it, and was a call every turn that came
back "pass". The seat is never asked to respond to its own spell or ability
while only its own are on the stack.

## A turn in one answer: `then`, `ask_again`, `attack`

In its own main phase, with the stack empty, the priority answer carries
three more fields, each offered only where it can mean something:

- `then` — cards from the hand to play or cast after `action`, in order,
  each taken without asking again once the stack is empty (targets are
  still asked). A pass with a plan starts the plan now: a model answering
  `{"action": 0, "then": ["Swamp"]}` meant "play the Swamp". The plan
  stops, and the seat is asked and told why, if the opponent puts anything
  on the stack (seen or not, #861) or a card cannot be played when its
  turn comes — at once, where the plan was started by a pass (#860).
- `ask_again` — false, the usual answer, ends the main phase after `action`
  and `then`: the seat passes until something happens, and the main-phase
  stops do not ask it that phase again. True asks again once they are done,
  for a draw spell to be played off. An empty plan used to mean "ask me
  again", and a seat that played its land and had nothing else to do was
  asked again only to pass.
- `attack` (first main phase, where a creature could attack now) — the `#`
  numbers of the creatures to attack the opponent with. At the declare
  attackers step the plan is declared without a call if nothing has happened
  since (any stop drops it, and so does anything the opponent cast or
  activated meanwhile, seen or not, #861), no planeswalker could be attacked instead, and
  every creature it names can attack; otherwise the seat is asked, told why.
  `[]` means "ask me at combat".

The log says `PLAN`, `PLAN_STEP`, `PLAN_DONE`, `PLAN_STOP`, `ATTACK_PLAN`
and `ATTACK_PLAN_DROPPED`. `reports/llm-cost.md` (phase 3) has what each
took out of a game's calls.

The row's text is its name alone. The stops are listed once, in the
system prompt's "Pass until something happens" section (`PASS_UNTIL_RULES`
in `llm.rs`), which is sent only to a seat that is offered the row
(`MTG_LLM_PASS_UNTIL=off` leaves out both, #755); a test holds that list
to the stops above (#756), and `reports/llm-cost.md` has what the row costs
per call (#757).

The row is the seat's own, not an engine action: the random seat, the CLI
and the page do not get it, and `surface_parity.rs` excludes it from the
CLI/LLM menu comparison on purpose, with a test that says why. The engine's
own auto-pass (an offer of only Pass, Concede and mana abilities is passed
without asking anyone) is unchanged and comes first.

In the `--log`:

- `AUTO_PASS [seat]` — `engaged at <turn, step>` when the row is picked,
  then `until: <what it is still waiting for> — <turn, step>, <stack>;
  passing since <where>, (N offers passed unasked)` for every offer passed
  without a call. `INFO`, one line per pass, so a stretch is visible.
- `AUTO_PASS_STOP [seat]` — `<what happened> (N offers passed unasked since
  <where>)`, and the next `PROMPT` opens with `After N priority offers
  passed for you unasked, the pass stopped: <what happened>.` above a
  `Recent events` recap carrying everything that happened meanwhile (the
  recap's 80-entry cap and its marker apply as always).
- `UNREADABLE [seat]` (`ERROR`) — a `claude -p` reply no JSON object could
  be read out of, as it came (its first 600 characters, the stop reason
  and the output tokens); the `REASK` after it asks again.
- `USAGE [seat]` (`DEBUG`) — one line per `claude -p` call: input, output,
  thinking, cache tokens and wall time, so the largest answer of a game can
  be read off the log. A call whose `cache_read` is 0 wrote the whole
  prefix again: after a game's first call of each kind of question, that is
  a schema or a system prompt that changed, and a bug.

`MTG_LLM_PASS_UNTIL=off` leaves the row out of every menu, for measuring a
run against one without it; unset, it is offered. The draft runner's games
use the same seat and get the row for free.

## One game

```bash
cargo build --release

# Free: watch two random seats play.
./target/release/mtg-runner --p1 random --p2 random --seed 7

# Free-of-API: an LLM seat on plan quota against random.
./target/release/mtg-runner --p1 claude-code --p2 random --log logs/smoke/game.log

# Play against an LLM seat yourself (needs a real terminal).
./target/release/mtg-runner --p1 cli --p2 claude-code:opus

# Metered. Only with intent.
ANTHROPIC_API_KEY=... ./target/release/mtg-runner --p1 claude:claude-haiku-4-5-20251001 --p2 random
```

`--p1` defaults to `cli` and `--p2` to `random`, so any scripted or piped run
must pass `--p1` explicitly — a `cli` seat with no tty is refused.

Other flags: `--deck1`/`--deck2`, `--seed <N>` (seeds shuffles and the random
seats, so a failure replays), `--on-the-play 1|2` (otherwise the opener is
randomized per CR 103.1), `--check-invariants` (structural checks at every
decision point, exit 2 on the first violation), `--quiet`, `--help`,
`--version`.

## Decks

`--deck1`/`--deck2` take a built-in name or a path to a deck file.

Built-in names (aliases in parentheses): `red-green` (`rg`), `white-black`
(`wb`), `blue-white` (`uw`), `black-aggro` (`ba`), `innistrad-white` (`iw`),
`innistrad-blue` (`iu`), `innistrad-green` (`ig`). The defaults are
`red-green` for seat 1 and `white-black` for seat 2.

Anything else is read as a deck file: one `COUNT CARD NAME` per line, `#`
comments and blank lines ignored, every name checked against the card
registry. The checked-in decks live in `decks/` (`gw-humans.txt`,
`rb-vampires.txt`, `ub-zombies.txt`, `ug-spider-spawning.txt`) and
`decks/coverage/` (one two-color file per pair, `wu-coverage.txt` and friends).

```bash
./target/release/mtg-runner --p1 random --p2 random \
  --deck1 decks/gw-humans.txt --deck2 decks/coverage/ub-coverage.txt
```

## What a seat is told about cards

The system prompt describes the seat's own decklist in full — cost, type
line, P/T and rules text for every card, both faces of a double-faced
card. In `mtg-runner` that is all it carries: the opponent's decklist is
hidden information, and a seat handed both lists knew from turn 1 what it
was playing against and, the list being exhaustive, what it was not
(issue #466). In `mtg-draft-runner` the system prompt also carries a
`## Card reference` for the whole set, which is public in a draft.

Everything else a seat learns about cards, it learns as they come into
view: each decision prompt carries an `Opp's cards in view:` section with
the rules text of every card on the battlefield, on the stack, in exile or
revealed that is not in the seat's own deck. A card that is only in a
graveyard is described where its text works from there (flashback and its
kin) or where the question or a stack object points at it (#864), and is
otherwise a name in the graveyard's list. It is re-sent on every prompt
while the card is in view and absent otherwise. The opponent's floating
mana is its own line, `Opp mana pool:` (#865).

## `--log`: what is recorded and how to read it

`--log <path>` appends the run log to the file, as the usage text says, so
a second run at the same path adds to the first — point each run at its own
path, under `logs/<run-name>/`, or count by the run's own `GAME_START`.

Every entry is one tab-delimited header line:

```
<timestamp>\t<LEVEL>\t<thread>\t<file>:<line>\t<LABEL>\t<content>
```

Multi-line content (a prompt, the system prompt, the result summary) leaves
the content field off the header and follows it as flush-left body lines, so
header rows are the ones starting with a timestamp digit.

`LEVEL` is `INFO`, `DEBUG`, or `ERROR`. The labels worth knowing:

- `GAME_START` — the seat/deck mapping, stated as `p0 (--p1)` / `p1 (--p2)`.
- `GAME` — the engine's own log, streamed as the game runs, so a killed run
  still holds the sequence that led there. Private entries (one player's
  hidden information) never reach the file.
- `SYSTEM` — the full system prompt sent to an LLM seat, once per seat.
- `PROMPT` / `THOUGHT` / `RESPONSE` / `CHOSE` — one decision exchange:
  the board state and options sent, the model's reasoning, the raw backend
  JSON (`DEBUG`), and the option that was taken. Every backend records a
  `THOUGHT`: the Messages API reads it from the extended-thinking block,
  while Gemini and the `claude -p` seat — whose result object carries no
  thinking block — ask for it in the JSON as a `thoughts` field and take it
  out of the payload once captured.
- `MALFORMED`, `VALIDATION`, `BLOCKER_VALIDATION` — the model answered with
  something unusable and the harness fell back (usually to option 0 / pass).
  Grep these first when a seat plays nonsense.
- `API_RETRY`, `API_ERROR`, `API_WARN`, `API_FATAL` — transport trouble;
  `claude -p` failures land here too.
- `COLLAPSED` — how many legal actions were folded into how many presented
  options (`DEBUG`).
- `AUTO_PASS`, `AUTO_PASS_STOP`, `USAGE` — the pass-until row and the
  per-call usage, above.
- `INVARIANT` — a `--check-invariants` violation.
- `RESULT`, `TOKEN_USAGE` — end of run.

Useful reads:

```bash
cut -f2,5,6 logs/smoke/game.log | grep -v '^DEBUG'      # the INFO story
awk -F'\t' '$5 ~ /MALFORMED|VALIDATION/' logs/smoke/game.log
grep -A40 $'\tPROMPT' logs/smoke/game.log               # a prompt with its body
```

Independently of `--log`, `mtg-runner` prints a per-model token line at the
end (calls, input, output — with the thinking share where the backend reports
it — cache read, cache create) and what it cost: dollars for a metered seat,
`n/a (plan quota; $X at API rates)` for a `claude-code:<model>` seat whose
model has a published rate, `unknown` for a model this build has no rate for.

## Saving and resuming

```bash
./target/release/mtg-runner --p1 claude-code --p2 random --save logs/smoke/game.save
./target/release/mtg-runner --p1 claude-code --p2 random --resume logs/smoke/game.save
```

The save is rewritten before every decision point via a temp file and
`rename(2)`, so a reader always sees a whole save. It is left in place when
the game ends, holding the final position; `--resume` on a finished save
refuses and says who won. A save that still holds a live game means the run
died or was quit.

A save is the whole game state, hidden information included: both hands,
both libraries in draw order, and the Private log lines (what a player kept
from a look-at-the-top effect). It has to be, to resume. It is written
readable by its owner only (mode 0600), but anyone who can read the file can
read the answer key — in a hotseat game, that includes the other player.

A save is written only when a `cli` seat is playing or `--save` was passed —
serializing the whole state every action is too expensive for a long
AI-vs-AI run to pay for nothing. So an unattended run that you might want to
resume needs `--save`. (A hot-reload snapshot is always written to a
per-process file in the temp directory for the CLI's own reload key; it is not
a substitute for `--save`.)

On `--resume`, the save's decks, RNG and seats win over the flags: `--deck1`
and `--deck2` are ignored and the runner says so. `--seed` does not change the
saved game: a random seat continues from where the save says its draws had
got to, so a resume plays on the game it was saved from (#844); `--seed`
seeds only a random seat a save written before that holds no position for. The seats are stored in the save, so a
bare `--resume` brings back the same players; an explicit `--p1`/`--p2`
overrides the saved seat and says so, and a metered API seat the file asks
for needs that flag to confirm it.

A save also holds each LLM seat's memory — its notes, how far into the game
log it has been shown, and a pass-until in progress — so a resumed seat is
asked exactly what it would have been asked had the game never stopped: the
same prompt, byte for byte, with the events since its last decision and no
recap (`mtg-runner/tests/save_files.rs` pins it). A save written before the
memory was kept resumes the seat the old way: a recap of the whole game log
in its first prompt (logged as `RESUME`) and the notes the save holds.

`--stop-after <N>` ends the run after the Nth decision either seat makes and
prints `DECISION <n> p<seat> <action as JSON>`. With `--resume` it replays one
saved position as one decision, which is what `scripts/decision-eval/` is
built on:

```bash
# Keep the position of every model call of a live game ...
EVAL_SAVE=$PWD/logs/eval/g1.save EVAL_POSITIONS=$PWD/logs/eval/positions \
EVAL_REAL_BIN=$(command -v claude) \
CLAUDE_CODE_BIN=$PWD/scripts/decision-eval/capture-claude.py \
  ./target/release/mtg-runner --p1 cc:claude-sonnet-4-6 --p2 random \
    --save logs/eval/g1.save --log logs/eval/g1.log
# ... replay them under two configurations, and score one against the other.
scripts/decision-eval/replay.py --positions logs/eval/positions --out logs/eval/res \
  --name ref --p1 cc:claude-opus-5-5 --env MTG_LLM_THINKING=high --samples 2
scripts/decision-eval/replay.py --positions logs/eval/positions --out logs/eval/res \
  --name off --p1 cc:claude-sonnet-4-6 --env MTG_LLM_THINKING=off
scripts/decision-eval/compare.py --out logs/eval/res --ref ref ref off
```

`compare.py` prints, per configuration, how often its action matches the
reference's (on the positions where every reference sample agreed, and on all
of them) and what a decision cost it in calls, output and thinking tokens and
prompt bytes. Every replayed decision is a real model call: on a `cc` seat
that is plan quota; `replay.py` refuses a metered seat — except under the
prompt stub, which is free and makes the same positions a byte-for-byte
measure of what two builds send:

```bash
scripts/decision-eval/replay.py --positions logs/eval/positions --out logs/eval/stub \
  --name before --bin /elsewhere/mtg-runner --p1 cc \
  --env CLAUDE_CODE_BIN=$PWD/scripts/llm-prompt-stub.py --env STUB_CALLS=/dev/null
```

## Drafting

```bash
mkdir -p logs/isd-draft
./target/release/mtg-draft-runner --players 8 --model cc --best-of 3 \
  --log logs/isd-draft/draft.log
```

`--set <name>` loads `data/sets/<name>.json`; `isd` is the only set checked in
and is the default. Cards the engine does not implement are dropped from the
pool with a warning.

`--model <spec>` sets every seat; `--model-<N> <spec>` overrides seat N
(0-based). It defaults to `claude` — a metered seat — so a run that does not
pass `--model` bills the API for all eight drafters. Defaults elsewhere:
`--players 8`, `--best-of 3`. The spec is `provider[:model[:draft_thinking[:game_thinking]]]`
with the same providers and the same billing as above — a `claude` or `gemini`
seat meters every pick, every deck-build retry, and every game decision, so an
eight-seat metered draft is a large bill. `claude-code`/`cc` runs the picks,
the deck build, and the games all through `claude -p`.

`--guide <path>` prepends a draft guide to every seat's prompt;
`--guide-<N> <path>` does one seat. An unreadable guide file is fatal.

`--best-of <N>` sets games per tournament match. `--log <path>` defaults to
`draft.log` in the working directory — always pass a path under `logs/`
instead. The log holds the packs, every pick with its prompt and response, the
deck builds, the match results, and the final standings; the run also prints
standings and a token-usage summary to stderr.

`--help` and `--version` answer and exit without drafting, an unknown flag or
a missing value is refused, and an unknown provider is refused by name — a
mistyped invocation costs nothing rather than starting an eight-seat draft.

Between the games of a match each seat is asked once, before game 2 and
before game 3: the score, how the last game went, the cards the opponent
has shown (public zones, tokens left out), and its pool with the counts it
is playing now. It answers in the deck build's shape plus `play_first`,
which is read only when the choice is its own — the last game's loser
chooses, and after a drawn game whoever chose before. Repeating the counts
keeps the deck; an answer that does not make a legal deck keeps it too,
with no retry. The log has `[Seat N] SIDEBOARD_PROMPT before game G`, its
`_RESPONSE`, and one `[Seat N] SIDEBOARD before game G: in: ...; out: ...`
line (or `no change`) with `; plays first` or `; draws first` for the
chooser. Game 1's player on the play is a coin flip's winner, who plays —
no seat is asked before game 1. The question is one cached schema for every
pool, like the deck build's, and costs at most two calls a seat a match;
the usage summary counts it under `Draft`.

`--seed <N>` fixes the packs, the shuffles and play/draw; a run without one
logs the seed it drew. `--save <path>` snapshots the run after every pick
round, the deck builds and every match, and `--resume <path>` carries on
from one: the recorded picks are replayed without a model call, the decks
are used as built and the finished matches are counted.

`scripts/draft-loop/night.sh` is this run as a nightly routine plays it:
eight `cc:claude-haiku-5-5` seats, best-of-3, the date as the seed, the
even seats drafting with `reports/draft-loop/guide.md` and the odd seats
without it. After the tournament each seat is asked for its lessons and
the guide is rewritten from them and every night's statistics;
`reports/draft-loop/README.md` says what each night keeps.

### What a pick sends, and the pick eval

A drafting seat's system prompt is the draft rules, its place at the table
and the set's whole card reference (`## Card reference`, every card's cost,
type line and rules text, ≈ 40 KB); it is the same bytes on every pick, so
after a seat's first call it is read from the prompt cache at a tenth of the
input price. Each pick prompt is the seat's notes from its last pick, the
pack's lines (cost, type, size, rarity) and the pool restated with its
colours and curve — about 2 KB. A pack's last card is taken without a call.
Dropping the reference for each pack card's own text cut a pick's tokens
sevenfold and its price by a quarter at most, and picked measurably worse;
`reports/llm-cost.md`, phase 4, has the numbers, and
`scripts/measure-draft-prompts.sh` counts what a stub draft sends.

A pick is a pure function of what the seat is shown, so a real draft's picks
can be asked again of another seat or build and the choices compared — the
draft's counterpart of the game eval above:

```bash
# Keep every asked pick of a real draft ...
MTG_DRAFT_POSITIONS=$PWD/logs/draft-eval/picks.jsonl ./target/release/mtg-draft-runner \
  --model cc:claude-haiku-5-5 --players 2 --best-of 1 --seed 21 -q --log logs/draft-eval/draft.log
# ... ask them again under a reference and a candidate, and score one against the other.
scripts/decision-eval/draft-replay.py --positions logs/draft-eval/picks.jsonl --out logs/draft-eval/res \
  --name ref --model cc:claude-opus-5-5 --env MTG_LLM_THINKING=high --samples 2
scripts/decision-eval/draft-replay.py --positions logs/draft-eval/picks.jsonl --out logs/draft-eval/res \
  --name haiku --model cc:claude-haiku-5-5
scripts/decision-eval/compare.py --out logs/draft-eval/res --ref ref haiku
```

`mtg-draft-replay` (what `draft-replay.py` runs, one position at a time)
refuses a metered seat without `--metered`, and `draft-replay.py` refuses
one outright. A pick replays as the prompt it was live on any build that
renders pick prompts as the drafting build did
(`mtg-draft-runner/tests/kept_picks_replay.rs`). A position records its
seat's guide, the `MTG_LLM_HISTORY` and the `MTG_CLAUDE_CODE_DRAFT_ANSWERS`
it was asked under, and is asked again under that guide;
`mtg-draft-replay --guide FILE` asks every position under one file instead.
A position whose system prompt the replay cannot make as it was live (kept
before guides were recorded, or replayed under another history or answer
format) is said on stderr, and `draft-replay.py` keeps it in the row as
`unlike_live` and counts it in a WARN at the end of the run (#920). The positions and reference
behind phase 4 are kept in `reports/decision-eval/2026-10-20-draft/`.

To draft *with* people — a hosted table where humans join from a browser
or a terminal and AI seats fill the rest, then everybody plays — see
`docs/draft-with-friends.md` (`mtg-draft-server` and `mtg-draft-client`,
in the same crate). Its AI seats are told exactly what the runner's are,
and its `--log` is the same format.

## Not supported yet

- **The spec's thinking suffix only reaches Gemini.** `with_thinking_level`
  is a no-op for the Anthropic and Claude Code backends, so
  `:draft_thinking:game_thinking` in a `mtg-draft-runner` spec changes nothing
  on a `claude` or `claude-code` seat — those take `MTG_LLM_THINKING` (above).
  Relatedly, `mtg-draft-runner` validates the level strings only for `gemini`
  specs; a bogus level on a `claude` spec is accepted and ignored.
- **`mtg-runner` has no thinking-level syntax at all.** It splits a seat spec
  at the *first* colon, so `--p1 gemini:gemini-2.5-flash:high` asks the API for
  a model literally named `gemini-2.5-flash:high` and fails. Only
  `provider[:model]` works there.
- **The provider is inferred from the model name in `mtg-runner`.** A model
  containing `gemini` builds a Gemini backend whatever provider word preceded
  it, so `--p1 claude:gemini-2.5-flash` demands `ANTHROPIC_API_KEY` at the
  gate and then calls the Gemini API. Name the provider that matches the model.
- **`mtg-draft-runner`'s cost summary** prints metered seats in dollars, a
  `claude-code` seat as `n/a (plan quota)` with no API-rate equivalent, and a
  model this build has no published rate for as `unknown` rather than a
  made-up figure.
