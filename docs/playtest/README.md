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

For game and CLI subjects, pick deck pairs from `decks/` and
`decks/coverage/`, or write one-off decks into a temp file (deck files
are `COUNT NAME` lines). Check the pairing can actually reach what you
came for before playing two games into a dead end — the ledger is full of
nights that ended in "no card in either deck can do this".

## Filing

You are a finder in the bug pipeline (`docs/plans/bug-pipeline.md`): you
file issues, you never fix. One issue per distinct defect, labels
`bug` + `phase:playtest`, title `[playtest] <short symptom>`, body with:

- **Found-by** — this crew, the date, the subject and the idea id (or a
  sentence describing the probe, if you invented it).
- **Target** — engine, machine or harness, per the pipeline's glossary.
- **Repro** — exact commands. A fresh reader with no context must be able
  to run them.
- **Evidence** — verbatim captures, log excerpts, exit codes, and the CR
  rule cited when there is one.
- **Confidence** — say how sure you are, and say when something might be
  deliberate.

Search open issues for the same symptom first and comment there rather
than duplicating. UX judgments are worth filing; label the severity
honestly.

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
