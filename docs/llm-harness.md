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

Default models when the spec names none: `claude` → `claude-sonnet-4-6`,
`gemini` → `gemini-2.5-flash`, `claude-code` → whatever the CLI defaults to
(no `--model` is passed).

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

## `--log`: what is recorded and how to read it

`--log <path>` writes the run log. Despite the word "Append" in the usage
text, the file is truncated when it is opened — point each run at its own
path, under `logs/<run-name>/`.

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
  the board state and options sent, the model's extended thinking if the
  backend returned any, the raw backend JSON (`DEBUG`), and the option that
  was taken.
- `MALFORMED`, `VALIDATION`, `BLOCKER_VALIDATION` — the model answered with
  something unusable and the harness fell back (usually to option 0 / pass).
  Grep these first when a seat plays nonsense.
- `API_RETRY`, `API_ERROR`, `API_WARN`, `API_FATAL` — transport trouble;
  `claude -p` failures land here too.
- `COLLAPSED` — how many legal actions were folded into how many presented
  options (`DEBUG`).
- `INVARIANT` — a `--check-invariants` violation.
- `RESULT`, `TOKEN_USAGE` — end of run.

Useful reads:

```bash
cut -f2,5,6 logs/smoke/game.log | grep -v '^DEBUG'      # the INFO story
awk -F'\t' '$5 ~ /MALFORMED|VALIDATION/' logs/smoke/game.log
grep -A40 $'\tPROMPT' logs/smoke/game.log               # a prompt with its body
```

Independently of `--log`, `mtg-runner` prints a per-model token line at the
end (calls, input, output, cache read, cache create). It prints token counts
only — no dollar figure.

## Saving and resuming

```bash
./target/release/mtg-runner --p1 claude-code --p2 random --save logs/smoke/game.save
./target/release/mtg-runner --p1 claude-code --p2 random --resume logs/smoke/game.save
```

The save is rewritten before every decision point via a temp file and
`rename(2)`, so a reader always sees a whole save. It is deleted when the game
ends normally, so a leftover file means the run died.

A save is written only when a `cli` seat is playing or `--save` was passed —
serializing the whole state every action is too expensive for a long
AI-vs-AI run to pay for nothing. So an unattended run that you might want to
resume needs `--save`. (A hot-reload snapshot is always written to a
per-process file in the temp directory for the CLI's own reload key; it is not
a substitute for `--save`.)

On `--resume`, the save's decks and RNG win over the flags: `--deck1`,
`--deck2`, and `--seed` are ignored and the runner says so. The seat flags are
not stored in the save — pass `--p1`/`--p2` again, and note that a resumed LLM
seat starts a fresh conversation seeded with a recap of the game log so far,
not the original conversation.

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

There is no `--seed`, `--save`, or `--resume` here: a draft run cannot be
replayed or resumed.

## Not supported yet

- **Thinking levels only reach Gemini.** `with_thinking_level` is a no-op for
  the Anthropic and Claude Code backends, so `:draft_thinking:game_thinking`
  in a `mtg-draft-runner` spec changes nothing on a `claude` or `claude-code`
  seat. Relatedly, `mtg-draft-runner` validates the level strings only for
  `gemini` specs; a bogus level on a `claude` spec is accepted and ignored.
- **`mtg-runner` has no thinking-level syntax at all.** It splits a seat spec
  at the *first* colon, so `--p1 gemini:gemini-2.5-flash:high` asks the API for
  a model literally named `gemini-2.5-flash:high` and fails. Only
  `provider[:model]` works there.
- **The provider is inferred from the model name in `mtg-runner`.** A model
  containing `gemini` builds a Gemini backend whatever provider word preceded
  it, so `--p1 claude:gemini-2.5-flash` demands `ANTHROPIC_API_KEY` at the
  gate and then calls the Gemini API. Name the provider that matches the model.
- **No cost line from `mtg-runner`**, only token counts. `mtg-draft-runner`
  prints a cost summary: metered seats in dollars, a `claude-code` seat as
  `n/a (plan quota)`, and a model this build has no published rate for as
  `unknown` rather than a made-up figure.
