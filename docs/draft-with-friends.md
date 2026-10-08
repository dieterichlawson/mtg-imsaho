# Draft with friends

`mtg-draft-server` hosts a booster draft on one machine that any mix of
people and AI seats sit at. People draft and build from a browser page or
a terminal client; AI seats draft and build through the same LLM code the
unattended `mtg-draft-runner` uses; then a Swiss tournament is played,
each person through the game page on a port of their own and each AI
seat through the LLM game seat. The design is `docs/plans/draft-with-friends.md`.

## Host a table

From the repository root (the page and `data/sets/` are read relative to
it; `MTG_GUI_DIR` points elsewhere for the page):

```bash
cargo build --release -p mtg-draft-runner
./target/release/mtg-draft-server --seats human,human,ai,ai --bind 0.0.0.0 \
    --best-of 3 --log logs/friday/draft.log
```

The server deals the packs from the seed, opens the log, and prints one
join line per human seat:

```
Innistrad draft: 4 seats, best-of-3, seed 1479...; log logs/friday/draft.log
seat 0  http://192.168.1.20:8800/?seat=0&key=7c5b0a8e...   (or: mtg-draft-client ws://192.168.1.20:8800/ws?seat=0&key=7c5b0a8e...)
seat 1  http://192.168.1.20:8800/?seat=1&key=3e0f...
the draft starts when everybody has joined, or press Enter to start without them
```

Send each friend their line. The key is the whole access control: a seat
is whoever holds its key, a second tab with the same key is the same seat
(the page is a view of the server's state, so a reconnect loses nothing),
and a wrong key is refused by name. `--bind` defaults to `127.0.0.1`,
which only this machine can reach; `--bind 0.0.0.0` listens for the LAN or
a tailnet, and the join lines name this machine's first non-loopback
address (or say to replace `0.0.0.0` with it when that cannot be found).

### Seats

`--seats` is a comma list in seat order (seat 0 passes left to seat 1):

- `human` — a person, through the page or the client.
- `ai` — an LLM seat under the default spec, `cc` (the plan-quota seat
  through `claude -p`; never a metered seat by default). `--ai <spec>`
  changes the default; `ai:<spec>` names one seat's, in the runner's
  `provider:model[:draft_thinking[:game_thinking]]` form, e.g.
  `ai:cc:claude-sonnet-4-6`. A `cc` seat needs the `claude` CLI on the
  path (or `CLAUDE_CODE_BIN`), and its default model may refuse the game
  prompt — name one.
- `cli` — the host's own seat at the terminal. Not implemented in this
  version: the server refuses it and says so. Sit at a `human` seat and
  open its link on this machine instead.

`Nx` prefixes repeat: `--seats 1xhuman,7xai`. Two to eight seats.

### Flags

| flag | default | |
|---|---|---|
| `--seats <list>` | `human,ai,ai,ai` | who sits where |
| `--ai <spec>` | `cc` | the spec a bare `ai` uses |
| `--set <name>` | `isd` | `data/sets/<name>.json` |
| `--bind <addr>` | `127.0.0.1` | `0.0.0.0` for friends on the network |
| `--port <N>` | `8800` | the lobby's port |
| `--game-ports <lo-hi>` | `8801-8899` | ports the humans' game pages take |
| `--best-of <N>` | `3` | games per match |
| `--seed <N>` | generated | packs, shuffles, play/draw; in the log header |
| `--guide <path>` | none | a draft guide prepended to every AI seat's prompt |
| `--pick-seconds <N>` | none | pick for a human who has not in N seconds |
| `--build-seconds <N>` | none | build for a human who has not in N seconds |
| `--log <path>` | `logs/draft-with-friends/draft.log` | the draft log; `decks/` and `games/` beside it |
| `--quiet` | | no event lines on the terminal |

`--help` and `--version` answer and exit; an unknown flag, a bad seat
word, an unknown provider, or a `cc` seat with no CLI is refused before
anything is dealt.

### The host's keyboard

The terminal prints a line per event — a join, every pick, every deck,
every pairing, every game and match result, the standings — and reads:

- Enter or `start`: start before everybody has joined. A seat that has
  not joined is picked for by the table until the person arrives, then
  it is theirs again.
- `kick <seat>`: hand a human seat to the table for good. It is picked
  for and built for, and its matches are forfeit (recorded as such in the
  log and the standings) rather than waited on.
- `status`: where everybody is.
- `quit`: stop the server. It stays up after the tournament so the pages
  can be read until you do.

## Join

### In a browser

Open your join link. `mtg-gui/draft.html` is the page (served from the
same `mtg-gui` directory as the game page, with `dist/` and `assets/`); it
shows the lobby, the pack with the pool beside it while drafting, a
checklist and land counts while building, and your match link, results
and standings while playing. If the page is not in the checkout yet the
server says so at start-up and the client below still works.

### In a terminal

```
mtg-draft-client ws://192.168.1.20:8800/ws?seat=0&key=7c5b0a8e... --name Lawson
```

A plain line-oriented client. It prints your view when something of
yours changes — the pack as a numbered list, your pool with counts — and
one line for the table when only other seats moved. Type a card's number
to pick it. While building: `add <n>` and `drop <n>` by the pool numbers
shown, `lands plains=7 island=10`, `show`, and `ready` when the deck is
final; the server's refusal (`Deck has 39 cards (need at least 40)`) is
printed as it comes. When a match is paired it prints `Round 1 vs seat 3:
playing — open http://192.168.1.20:8803/`. `name <x>`, `help`, `quit`.

## Draft, build, play

Drafting is asynchronous: each seat has a queue of packs, a pick passes
the rest of the pack to the neighbour (left for packs 1 and 3, right for
pack 2), and a seat that is slower than its upstream neighbour sees
"1 pack waiting". A round ends when every pack of it is empty; the next
round's packs are dealt to everybody at once. Nobody waits on anybody who
is not holding their next pack.

Deck building starts for a seat as soon as its own draft is over. A deck
is any cards of your pool plus basic lands, 40 or more; the server
validates with the runner's `validate_deck` and refuses an illegal deck
with the reason. `ready` makes it final. Every deck is written to
`<log dir>/decks/seat-N.txt` as `COUNT NAME` lines, which
`mtg-runner --deck1` plays later. An AI seat builds through the runner's
LLM deck builder, with the runner's fallback deck on failure (marked in
the log and the standings, as the runner marks it).

When every deck is in, a Swiss tournament pairs the rounds as the runner
does. For each match a person is in, a game page is bound on the next
free port in `--game-ports` and the link is shown on your page and in the
client; a match between two people is two pages on two ports; an
AI-vs-AI match plays on its own. Results and standings appear as they
come; the next round is paired when the round is over.

## The log and the files

`--log` is the runner's draft log format, so `docs/playtest/drafting.md`'s
readers apply: the header (seats and models, with `human` for a person,
the seed), the booster packs, every pick — an AI seat's with its prompt
and response, a person's with `human` where those go, the table's with
`auto-pick: <why>` — the pools, the deck builds, the matches with their
game logs, the standings. A `NOTE` after the header says the table is
asynchronous, so picks are in the order they happened. Beside the log:
`decks/seat-N.txt` and `games/r<round>-<a>v<b>-g<n>.log`.

`GET /api/view?seat=N&key=K` returns the seat's view as JSON, which is
what the client and the tests read; the page gets the same object over
the WebSocket at `/ws?seat=N&key=K` after every change.

## Limitations

- No snapshot or resume: `--resume` is accepted, noted in the log, and
  ignored. A server that dies takes the draft with it.
- No `cli` seat (see above).
- `kick` does not interrupt a game already in progress: a page nobody is
  at waits, as the game runner's does. Kick before the round.
- Spectators, chat, cubes, more than one set, and anything past the key
  for access control are out of scope.
- The browser page is a separate piece of work; until it lands in
  `mtg-gui/`, `GET /` is a 404 that says so, and the terminal client is
  the way in.
