# The Innistrad draft loop

Every night a table of eight AI drafters drafts Innistrad, builds, and
plays a best-of-three Swiss tournament; then each drafter writes down what
it learned, and the guide the next night drafts with is rewritten from
those lessons and every night's statistics. Over many nights the guide is
meant to become the best Innistrad draft guide these drafters can write,
and the ledger says whether it is helping.

`scripts/draft-loop/night.sh` runs a night; the nightly routine runs it and
commits what is here.

## What is here

| file | what |
| --- | --- |
| `guide.md` | The guide. Even seats draft with it in their prompt at every pick; odd seats without it. `git log -p guide.md` is its history. |
| `stats.md` | Every night's tables: guided against unguided seats, colour pairs, and per card how early it is taken, how often it is played, and how its decks do. `stats.json` is the same as data. |
| `nights/DATE.json` | One night as data: each seat's picks (pack, pick, card), colours, deck, record, end-of-draft notes, and every pack it was handed. |
| `nights/DATE-lessons.json` | Each seat's lessons, with its record and whether it had the guide. |
| `nights/DATE-curate.json` | The guide's rewrite: the model, its length, and whether it was taken. |
| `nights/DATE.md` | The night, for reading: standings, guided and unguided records, lessons, the guide's fate, what it spent. |
| `nights/DATE-positions.jsonl.xz` | Every pick a seat was asked, as `MTG_DRAFT_POSITIONS` keeps it, each with its seat's guide: the pick eval's input (`scripts/decision-eval/draft-replay.py`). |

## How a night runs

1. `mtg-draft-runner --model cc:claude-haiku-5-5 --players 8 --best-of 3
   --seed <date> --guide-0 guide.md --guide-2 ... --save ...` — the draft,
   the deck builds, sideboarding between games, and the tournament.
2. `ledger.py night` and `ledger.py stats` — the night into the ledger,
   and the tables over all nights.
3. `reflect.py debrief` — one `claude -p` call per seat: its picks, deck,
   matches (each opponent's deck and the game lengths), its own notes, and
   the guide it drafted with, asked for up to three lessons grounded in
   what happened.
4. `reflect.py curate` — one call (Sonnet 5.5) with the guide, the lessons
   and the tables, asked for the next guide: six fixed headings, at most
   700 words, numbers with their sample sizes, no card the data does not
   name. An answer that is not a guide is tried once more, then the old
   guide stands.
5. `report.py` — `nights/DATE.md`, with what went wrong (re-asks, replies
   no answer could be read from, panics, substituted picks and decks)
   counted from the run's log.

A night that stopped partway carries on from its snapshot when
`night.sh` is run again with the same date; a date already in the ledger
is refused, and a second night that day is `DATE_b`.

Every seat is `claude -p` on the CLI's own login: plan quota, never a
metered key. `DRAFT_LOOP_MODEL`, `DRAFT_LOOP_CURATOR`, `DRAFT_LOOP_PLAYERS`
and `DRAFT_LOOP_BEST_OF` change the defaults; `DRAFT_LOOP_OUT` and
`CLAUDE_CODE_BIN` (the prompt stub) make a dry run that writes elsewhere.

## Reading it

The guided-against-unguided table is the loop's own test. A night is eight
decks and about thirty games, so a difference of a game or two is noise;
read it over weeks. Per-card numbers below about ten games are anecdotes,
and the guide is told to say so.
