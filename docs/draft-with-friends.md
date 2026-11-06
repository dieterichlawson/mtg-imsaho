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

Send each friend their line. Everything a friend opens — the draft page,
the client's socket, and later their game page, at `/game/<port>/<key>/` — is on
the lobby's one port, so that is the only port to open: on a LAN,
`--bind 0.0.0.0`; across the internet, forward `--port` on the router, or
put the machine on a tailnet (Tailscale and the like: `--bind 0.0.0.0` and
give friends the tailnet address), or tunnel it (`ssh -R`, a Cloudflare
tunnel) and hand out the tunnel's address in place of the join line's
host. A friend's game link is on whatever host they joined by, so the
tunnel's or tailnet's address carries their games too (#969). The game
pages themselves listen on this machine's loopback only.

The key is the whole access control: a seat
is whoever holds its key, a second tab with the same key is the same seat
(the page is a view of the server's state, so a reconnect loses nothing),
and a wrong key is refused by name. A game page has a key of its own in
its link, so the page (the person's hand, their decisions) is not reached
by guessing its port. `--bind` defaults to `127.0.0.1`,
which only this machine can reach; `--bind 0.0.0.0` listens for the LAN or
a tailnet, and the join lines name this machine's first non-loopback
address (or say to replace `0.0.0.0` with it when that cannot be found).

### Seats

`--seats` is a comma list in seat order (seat 0 passes left to seat 1):

- `human` — a person, through the page or the client.
- `ai` — an LLM seat under the default spec, `cc:claude-haiku-5-5` (the
  plan-quota seat through `claude -p`, on the model that picked and
  played closest to a stronger model's choices for its cost —
  `reports/llm-cost.md`, phases 3 and 4; never a metered seat by
  default). `--ai <spec>` changes the default; `ai:<spec>` names one
  seat's, in the runner's `provider:model[:draft_thinking[:game_thinking]]`
  form, e.g. `ai:cc:claude-sonnet-5-5`. A `cc` seat needs the `claude`
  CLI on the path (or `CLAUDE_CODE_BIN`). An `ai:claude:` or `ai:gemini:`
  seat bills your `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` per call: start-up
  and `status` say so beside the seat, and one whose key is not set is
  refused before anything is dealt.
- `cli` — the host's own seat, at the server's terminal (at most one).
  The server runs the terminal client (below) for it against its own
  port: every line the host types that is not one of the host's commands
  (`start`, `kick`, `status`, `quit`, `help`) is the seat's — a pick
  number, `add`, `lands`, `ready`, `review`, `play`/`draw` between games.
  The seat is at the table before the keyboard is read, and its games are
  played on the game page whose link it prints, as a person's are. Its
  join link is printed too, so the draft page can be opened beside it.
  `--quiet` keeps the table's own lines out of the seat's screen; the
  answers to the host's commands are printed either way.

`Nx` prefixes repeat: `--seats 1xhuman,7xai`. Two to eight seats.

### Flags

| flag | default | |
|---|---|---|
| `--seats <list>` | `human,ai,ai,ai` | who sits where |
| `--ai <spec>` | `cc:claude-haiku-5-5` | the spec a bare `ai` uses |
| `--set <name>` | `isd` | `data/sets/<name>.json` |
| `--bind <addr>` | `127.0.0.1` | `0.0.0.0` for friends on the network |
| `--port <N>` | `8800` | the lobby's port |
| `--game-ports <lo-hi>` | `8801-8899` | loopback ports the humans' game pages take; the links reach them through `--port` |
| `--direct-game-ports` | | link each game page on its own port, bound on `--bind`, instead (every port in the range must then be reachable) |
| `--best-of <N>` | `3` | games per match |
| `--seed <N>` | generated | packs, shuffles, play/draw; in the log header |
| `--guide <path>` | none | a draft guide prepended to every AI seat's prompt |
| `--pick-seconds <N>` | none | pick for a human who has not in N seconds (1 to 86400) |
| `--build-seconds <N>` | none | after N seconds building, a human's deck is final: the one they last sent (basics fill a short one, up to 18 lands), or else the table's (1 to 86400) |
| `--sideboard-seconds <N>` | `120` | between games, a person has N seconds to sideboard and, when it is theirs, to choose play or draw; then the deck stands and the chooser plays (1 to 86400) |
| `--log <path>` | `logs/draft-with-friends/draft.log` | the draft log; `decks/` and `games/` beside it |
| `--save <path>` | `table.save.json` beside the log | the table's snapshot, rewritten after every change |
| `--resume <path>` | none | carry on from a snapshot (below) |
| `--quiet` | | no event lines on the terminal |

`--help` and `--version` answer and exit; an unknown flag, a bad seat
word, an unknown provider, or a `cc` seat with no CLI is refused before
anything is dealt.

### The host's keyboard

The terminal prints a line per event — a join, every pick, every deck,
every pairing, every game and match result, the standings — and reads:

- Enter or `start`: start before everybody has joined. A seat that has
  not joined is picked for by the table until the person arrives, then
  it is theirs again — on the pick timer when `--pick-seconds` is set
  (so somebody a minute late has lost a few picks, not the draft), at
  once when it is not. The deck likewise, under `--build-seconds`.
- `kick <seat>`: hand a human seat to the table for good (your own seat,
  at this terminal, takes `kick <seat> yes`). It is picked
  for and built for, and its matches are forfeit (recorded as such in the
  log and the standings) rather than waited on — a match it is playing
  too: within a couple of seconds its page stops being asked, the game in
  progress and the rest of the match go to the opponent, and the round
  carries on — whichever of the two is deciding (against an AI seat, once
  the AI's answer is in).
- `status`: where everybody is — "here", "away", "in game", "game page
  closed", "kicked", "not answering" — and what they are doing, and each
  AI seat's model (and what a metered one bills). A person
  in a match is wherever their game page is.
- `quit`, or Ctrl-C: stop the server. Every page and client is told the
  table is closed and why, and stops reconnecting; the log ends with
  `NOTE the host quit` (or `NOTE the server was stopped by Ctrl-C
  (SIGINT)`) and the usage summary. It stays up after the tournament so
  the pages can be read until you do.

While a match is on, closing a game page is said: the opponent's game
page shows "<name>'s game page is closed: the game waits for them to open
it again" where it would say "Opponent is thinking", both draft pages
show the seat as "game page closed", and the host's terminal says so.
A page not opened yet is said as that ("<name>'s game page is not open
yet", "game page not opened yet"), and its first open is not "open again".
Reopening the link carries on from the decision that was waiting. Each
game page also says which game of the match it is and the score.

## Join

### In a browser

Open your join link. `mtg-gui/draft.html` is the page (served from the
same `mtg-gui` directory as the game page, with `dist/` and `assets/`); it
shows the lobby, the pack with the pool beside it while drafting, a
checklist and land counts while building, and your match link, results
and standings while playing. Once your own draft is over, "Review your
picks" lists every pack you were handed, by pack, with the card you took
over the rest of it. On a phone, a tap on a pool card while
building opens its text under it and a second tap moves it between main
and side. If the page is not in the checkout yet the
server says so at start-up and the client below still works.

### In a terminal

```
mtg-draft-client ws://192.168.1.20:8800/ws?seat=0&key=7c5b0a8e... --name Lawson
```

A plain line-oriented client. It prints your view when something of
yours changes — the pack as a numbered list under one line counting your
pool (`pool` lists it), so the pack and the question are the last thing on
screen — and one line for the table when only other seats moved (while a pack is in
front of you, only when your queue of packs changes), which also says how
many more packs are waiting for you. Type a card's number
to pick it; a number typed before a new pack has been up for 300 ms (a
pick typed twice) is not taken from it, and the client says so. While building: `add <n>` and `drop <n>` by the pool numbers
shown, `lands plains=7 island=10`, `show`, and `ready` when the deck is
final; `text <n>` prints a card's rules text (the pack's card while
drafting, the pool's while building); the server's refusal (`Deck has 39 cards (need at least 40)`) is
printed as it comes. When a match is paired it prints `Round 1 vs seat 3:
playing (game 1)`, with the game's link on a line of its own under it, `open
http://192.168.1.20:8800/game/8803/<key>/`: through the table's own port and
carrying the page's key, which a bare game port without one refuses; a round you sit out is
`Round 2: a bye — counted as a match won`, and the seats show you as
`bye` (on the page too). With no match left to play that round, a bye or
yours finished, it says `Waiting for the others to finish their matches.`,
as the page does. Between games it prints `Before game 2 vs seat 3
(they lead 1-0):` with who chooses, what the opponent has shown and your
pool: `play` or `draw` when the choice is yours, the same `add`, `drop` and
`lands` to sideboard (a deck is sent once it has 40 cards), and `ready`.
`review` prints every pack you were handed and what you took from it.
`name <x>`, `help`, `quit`.

## Draft, build, play

Drafting is asynchronous: each seat has a queue of packs, a pick passes
the rest of the pack to the neighbour (left for packs 1 and 3, right for
pack 2), and a seat that is slower than its upstream neighbour sees
"1 pack waiting". A round ends when every pack of it is empty; the next
round's packs are dealt to everybody at once. Nobody waits on anybody who
is not holding their next pack.

Deck building starts for a seat as soon as its own draft is over. A deck
is any cards of your pool plus basic lands, 40 or more; the server
validates with the runner's `validate_deck`. A deck that is still short
is kept as work in progress with what it needs (the page and the client
send the deck after every card moved, so that is most of them); a deck
naming a card you did not draft, or a land that is not basic, is refused
with the reason. `ready` makes a deck final, and is refused while it is
short. Under `--build-seconds` the page and the client show the clock;
when it runs out the deck as you last sent it is final, a short one
filled with the basic it plays most of (up to 18 lands), and the table
builds its own only when you sent nothing it can play, saying why.
Every deck is written to
`<log dir>/decks/seat-N.txt` as `COUNT NAME` lines, which
`mtg-runner --deck1` plays later. An AI seat builds through the runner's
LLM deck builder, with the runner's fallback deck on failure (marked in
the log and the standings, as the runner marks it).

When every deck is in, a Swiss tournament pairs the rounds as the runner
does. For each match a person is in, a game page is bound on the next
free loopback port in `--game-ports` (counting on from the last one used,
round to the start; a finished match's port is let go) and the link —
`http://<host>:<port>/game/<its port>/<key>/`, through the lobby, with a key
of the page's own that only that person's link carries — is shown on
your page and in the client; once the match is over the link says so; a match between two people is two pages on two ports; an
AI-vs-AI match plays on its own. Results and standings appear as they
come; the next round is paired when the round is over.

### Between games

Matches are played as a tournament plays them (MTR 2.2 and 2.4). Before
game 1 a coin flip's winner chooses to play or draw; before each later
game the last game's loser chooses (after a game with no winner, drawn
or abandoned at the action budget, whoever chose before), and both players may sideboard: any deck of their pool with 40 or
more cards, starting from the deck they played last game. The draft page
shows a panel for it under "Your match is on" — the choice, the pool as
rows to move between deck and sideboard, the land steppers, the cards the
opponent has shown in the match, the clock, and "Ready for game N" — and
the game page says "Before game N: ... where you drafted" (the host's own
seat: "at the server's terminal") to the seats
with something to do. The next game starts when both are ready, or when
`--sideboard-seconds` runs out: the decks stand as they are, and a chooser
who never chose plays. An AI seat answers in one call (its notes, the
score and what its opponent showed); a seat the table plays for keeps its
deck and plays. The host's terminal says who sideboarded and who chose
what. The log has each AI seat's prompt and answer, and for every seat one
`[Seat N] SIDEBOARD before game G:` line once both are ready: the cards in
and out (or `no change`, or `kept the deck (<why>)` for a seat that could
not choose), and the chooser's ` | plays first`, ` | draws first` or ` | plays
first (no choice was made)`; before game 1 only the chooser's. Card names
are joined by `; ` and the parts by ` | `, since a name may hold a comma.

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

## When the server stops

The server rewrites its snapshot (`table.save.json` beside the log, or
`--save`) after every change, and prints where at start-up. If it stops —
a crash, Ctrl-C, a reboot — start it again with `--resume <snapshot>`
(and the same `--log`, so the log carries on): the seats, their names and
their join links are the ones everybody already has, every pick, deck and
finished match is replayed onto packs dealt again from the same seed, the
AI seats keep their notes, and the draft or the tournament carries on. A
match that was in progress is played again from its first game. The
snapshot's seed, set, seats and match length win over the flags, and are
checked as the flags would be. `--guide` and the `MTG_LLM_HISTORY`,
`MTG_CLAUDE_CODE_DRAFT_ANSWERS`, `MTG_CLAUDE_CODE_ANSWERS` and
`MTG_LLM_PASS_UNTIL` settings are not: pass `--guide` again on the resume.
The snapshot records the guide (its path and a digest of its contents),
the settings the AI seats were asked under and a digest of the set data
(its name, card text and rarities), and a resume under any other
prints `WARN --resume …: its system prompt is not the live one: …` on the
terminal and in the log. The snapshot records a digest of the whole
deal and each pick the card it took, so a build whose card pool has
changed since (a `git pull` and a rebuild deal other packs from the same
seed) refuses the snapshot, even when every card taken so far is where it
was, rather than handing anybody another pack; so does a snapshot whose decks, matches
or seats the table could not have written.

## Limitations

- Spectators, chat, cubes, more than one set, and anything past the key
  for access control are out of scope.
