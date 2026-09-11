# Playtesting

You are a tester. Tonight you are going to try to find something wrong
with this program by using it.

This directory is what the nightly playtest routine reads. Start here,
pick a subject, read that subject's guide, and go.

## Pick a subject

| Subject | What it is | Guide |
|---|---|---|
| The game | The rules engine — does a game played here follow the Comprehensive Rules? | [`playing.md`](playing.md) |
| The CLI | The binaries as programs — the TUI, flags, files, signals, terminals | [`cli.md`](cli.md) |
| The harness | The LLM interface — what a model in a seat is told, offered, and understood to have said | [`harness.md`](harness.md) |
| Drafting | `mtg-draft-runner` — packs, picks, deck building, the tournament | [`drafting.md`](drafting.md) |

This list is not closed. If you find something that is none of these —
the card implementations as a body of code, the save format, the deck
files, something nobody has named yet — that is a subject too. Write a
guide for it and add a row.

Choosing: `reports/playtests/LEDGER.md` records every night ever played.
Prefer the subject that has gone longest without one, and inside it
prefer ideas nobody has tried, then ideas nobody has tried lately (two
weeks is a reasonable floor unless you are re-probing a fresh fix). One
subject a night is normal; two or three probes within it is a night's
work.

**The floor is per idea, not per subject, and "already played" is a fact
to look up rather than infer.** The ledger's Mission column starts with
the id, so this says which ids in a series have ever been run:

```
grep -oE '\| D[0-9]+' reports/playtests/LEDGER.md | grep -oE 'D[0-9]+' | sort -uV
```

Compare it against the ids in that subject's guide before concluding the
series is spent. Ideas are not played in order and a series is never used
up because its low numbers were: on 2026-09-08 the crew recorded that
"C1-C37, L1-L39, V1-V38, M1-M5, H1-H5, D1-D7" had all been played inside
the two-week floor and skipped drafting on that basis, when the ledger
held D1, D2, D3, D5 and D7 only — D4 and D6 had never been run, and D8,
D9 and D10 were sitting in `drafting.md` unread. Drafting had by then
gone longer without a night than any other subject, which is the case the
first rule exists to catch.

## One decision, three surfaces

The subjects above are separate nights, but they are not separate code.
Every decision the engine asks for is presented three times — the CLI
screen, the LLM seat's prompt and response schema, and the random seat
the fuzzer plays — and a change to one of them is a question about the
other two.

So when a probe finds something in a prompt, ask the same question of the
other surfaces before you write it up. It costs a minute and it doubles
what the night is worth:

- A screen that clips or a menu that explodes is a *token* flood on the
  LLM side, where nothing wraps and nothing pages: the action list is one
  comma-joined line with no cap.
- A prompt the CLI renders fine may be a schema the API refuses outright
  (#398), and the harness turns that into an empty answer that reads
  exactly like a seat declining. Nothing in a CLI game can show you this.
- A prompt both readable surfaces answer well may still be one the random
  seat answers with a constant, which is how a whole class of resolution
  stops being fuzzed without any test failing.

File them as separate issues against the right target — see the glossary
under "Filing" below — but find them in one sitting.

## What a guide is, and isn't

Each guide has three parts: where the code and the contract live, how to
tell right from wrong for that subject, and a list of ideas.

**The ideas are not a syllabus.** They are what previous nights happened
to think of. The bugs that mattered most were usually not on the list
when the night began — they turned up because someone read the code,
read the rule it was supposed to satisfy, and noticed the two disagreed.
Working through the list is a fine way to spend a night; treating it as
the boundary of what could be wrong is not.

So: read the implementation before you play it. Read the rule, the CR
section, or the contract it is meant to satisfy. Go looking for the
disagreement. When you find a way to look that the list doesn't have,
take it, and then add it.

## Seats, and the cost rule

NEVER spawn a metered API seat. `--p1 claude`, `--p2 claude`,
`--p1 gemini`, `--p2 gemini` (any model suffix) call metered external
APIs and are forbidden without exception, on every subject. Your seats
are `cli` (driven by you through tmux), `random`, and `claude-code`
(`cc`) — the same LLM seat run through `claude -p`, billed to the CLI's
own login. There is no `scripted` seat, so don't plan around one.

Most nights want two `cli` seats. Only the harness and draft subjects
need `cc`, where it is the thing under test.

## Setup

```
cd /home/user/mtg-imsaho   # or wherever the repo is checked out
git pull
cargo build --release -p mtg-runner
mkdir -p logs/playtest
```

Drive the binary through tmux — the `play-cli` skill documents the house
patterns. Run with `--log logs/playtest/<game>.log` and `--save` so
anomalies leave a resumable snapshot.

**If more than one tester is running at once, give each one its own tmux
socket** (`tmux -L <mission-id> new-session …`, and `-L` on every
`send-keys`/`capture-pane` after it). The `play-cli` skill opens with
`tmux kill-session`/`kill-server` as a cleanup step; on the shared default
socket that kills every other tester's game mid-hand. On 2026-09-05 four
crews did this to each other and eight games had to be restarted. For the
same reason, target processes by `--seed` rather than `pgrep -x
mtg-runner | tail -1`, which will find somebody else's game, and treat
`logs/playtest/` as shared — do not wipe the whole directory, only your
own files.

For game and CLI subjects, pick deck pairs from `decks/` and
`decks/coverage/`, or write one-off decks into a temp file (deck files
are `COUNT NAME` lines). Check the pairing can actually reach what you
came for before playing two games into a dead end — the ledger is full of
nights that ended in "no card in either deck can do this".

## Filing

You are a finder: you file issues, you never fix. Three words that are
easy to confuse, fixed here, and used in every issue's **Target** line:

- **the engine** — the rules (`mtg-engine`).
- **the machine** — the binaries as programs: the CLI/TUI, flags, files,
  signals, save/resume, pack generation (`mtg-runner`, `mtg-draft-runner`,
  `mtg-player`'s interactive surface).
- **the harness** — the LLM interface: the prompts, the response schema and
  the conversation an LLM seat plays a game through (`mtg-player/src/llm.rs`
  and its backends). Documented in `docs/llm-harness.md`.

One issue per distinct defect, labels `bug` + `phase:playtest`, title
`[playtest] <short symptom>`, body with:

- **Found-by** — this crew, the date, the subject and the idea id (or a
  sentence describing the probe, if you invented it).
- **Target** — engine, machine or harness, per the glossary above.
- **Repro** — exact commands. A fresh reader with no context must be able
  to run them.
- **Evidence** — verbatim captures, log excerpts, exit codes, and the CR
  rule cited when there is one.
- **Confidence** — say how sure you are, and say when something might be
  deliberate.

Search open issues for the same symptom first and comment there rather
than duplicating. UX judgments are worth filing; label the severity
honestly.

You are one of three finders, and the labels tell them apart: the
`nightly-fuzz` workflow files `phase:fuzz` issues, one per failing seed
(`[fuzz] <pair> seed <N>: <violation>`, which doubles as its dedupe key);
the `weekly-mutants` workflow files `phase:mutants` issues for survivors
beyond `reports/mutants-accepted.txt`; you file `phase:playtest`. Every
issue also carries `bug`. One fixer — the "Daily bug fixer" routine —
works every open `phase:*` issue oldest-first: reproduce, root-cause, fix
the mechanism (never a per-card special case), regression test, merge to
master, close citing the commit. Its **Repro** section is what the fixer
starts from, which is why it has to be runnable by a fresh reader.

## Afterwards

Write the night's report to `reports/playtests/YYYY-MM-DD.md` and append
one ledger row per probe. The idea id carries the subject (C and L are
the game, V and M the CLI, H the harness, D drafting), so the existing
columns still work.

Then add to the guide what the night taught you — see below. Finally
clean up: `tmux kill-server`, and delete `logs/playtest/`; logs are
gitignored and never committed.

## Adding an idea

Any agent may add one — a tester mid-night, the fixer after a fix,
anyone reading the code who notices a hole. There is no queue and no
approval step: a new idea goes straight into the relevant guide with the
next free id in that letter series and a provenance tag that stays until
someone tries it.

```
- H6 [proposed 2026-09-04, from #147] hidden information in the resumed
  seat's recap: ...
```

Because untried ideas are picked first, adding one is the same as
scheduling it. Four rules keep the guides worth reading:

1. **Write something to do, not a topic.** Say what to set up, what to
   do, and what to verify. "Check trample more" schedules nothing.
2. **Cite what prompted it** — an issue number, a ledger row, a CR rule,
   the file you were reading when you noticed. An idea nobody can trace
   back to an observation is a guess.
3. **Extend before you add.** If it is a wrinkle on an existing idea, add
   the wrinkle there. A guide of 80 near-duplicates is worse than one of
   40 distinct ones.
4. **Commit it on its own**, so the addition is reviewable as a change.

Dropping an idea is the same move in reverse: if it has been tried
several times and never found anything, say so in the commit and remove
it. The guides are meant to churn.

The letter series are per subject and global: C and L in `playing.md`, V
and M in `cli.md`, H in `harness.md`, D in `drafting.md`. A new subject
picks an unused letter.
