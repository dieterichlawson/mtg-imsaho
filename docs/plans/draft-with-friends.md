# Draft with friends

A booster draft hosted on one machine that any number of humans and AI
seats sit at. Humans draft and build their deck from a browser page or a
terminal client; AI seats draft and build through the same LLM code the
unattended draft runner uses; the matches are then played through the
existing game surfaces (the browser page for a human, the LLM seat for
an AI). Written 2026-10-08, before the first line of it existed.

## What exists today, and what is missing

`mtg-draft-runner` drafts a pod of LLM seats in lockstep: every seat is
asked for its pick, all picks are applied, the packs rotate
(`mtg-draft/src/draft.rs`, `DraftState`). Then each seat builds a deck
through the LLM (`build_deck_with_llm` in `mtg-draft-runner/src/main.rs`),
and a Swiss tournament is played in-process (`mtg-draft/src/tournament.rs`,
`play_match`/`play_game` in `main.rs`) with `LlmPlayer` seats.

Nothing in it has a human. The game surfaces do: `mtg-runner --p1 gui`
serves `mtg-gui/` on a local port and plays one seat through a browser
(`mtg-player/src/gui.rs`: a hand-rolled HTTP + WebSocket server on
`tungstenite`, one process, one seat, one port, reconnect-safe because
every message is the whole pending decision), and `--p1 cli` plays a seat
in the terminal.

So the missing pieces are: a draft that does not run in lockstep (a
human takes thirty seconds, an AI takes ten, and nobody should wait on
anybody who is not holding their next pack); a way for a human to see a
pack, pick from it, and build a deck, from a browser or a terminal; and a
host that wires the drafted decks into matches humans can play.

## The shape

One new binary, `mtg-draft-server`, in the `mtg-draft-runner` crate
(`src/bin/mtg-draft-server.rs`), beside a terminal client
`mtg-draft-client` (`src/bin/mtg-draft-client.rs`). The crate grows a
`src/lib.rs` so both binaries and `main.rs` share `llm_client`,
`card_lines`, `draft_log`, the LLM deck builder and the game-player
factory (those last two move out of `main.rs` into lib modules; `main.rs`
keeps its behaviour and its tests).

```
mtg-draft-server --set isd --seats human,human,ai,ai:cc:claude-sonnet-4-6 \
    --bind 0.0.0.0 --port 8800 --best-of 3 --log logs/friday/draft.log
```

prints one join line per human seat:

```
seat 0  http://192.168.1.20:8800/?seat=0&key=k3Qm...   (or: mtg-draft-client ws://192.168.1.20:8800/ws?seat=0&key=k3Qm...)
seat 1  http://192.168.1.20:8800/?seat=1&key=...
```

and the host sends each friend their line. A seat is claimed by its key;
a second tab with the same key sees the same seat (the page is a view of
server state, as the game page is). A key is a random 128-bit string
printed once; it is the whole access control and that is enough for a
LAN or a tailnet.

### Seat specs

`--seats` is a comma list, one entry per seat, in seat order (seat 0 passes
left to seat 1):

- `human` — a person, joining through the page or the client.
- `ai` — the default AI spec (`cc`, the plan-quota seat, as the runner's
  `--model` default is `claude`; the host is a person at a keyboard and
  should not be billed by default). `ai:<model spec>` names one, in the
  runner's `provider:model[:draft_thinking[:game_thinking]]` form.
- `cli` — the host's own seat: drafts and builds through the terminal the
  server runs in, and plays its games through the terminal CLI seat.
  At most one; the server's own progress lines go to the log, not the
  terminal, while a `cli` seat is at the keyboard. (Should-have; the
  page covers the host too.)

Pod size is the list's length; 2 to 8, like the runner's `--players`.
`--seats 1xhuman,7xai` is sugar worth having.

### Phases

The server is a state machine with four phases and a terminal one:

1. **Lobby.** Packs are generated from `--seed` the moment the server
   starts (the draft log records them, as the runner's does). The draft
   starts when every human seat has joined, or when the host presses
   Enter / sends `start` (an unjoined human seat is then auto-picked
   until it joins; see timers). AI seats are always present.
2. **Drafting.** Asynchronous. Each seat has a queue of packs in front of
   it. A seat with a non-empty queue is offered the head pack; its pick
   removes one card and passes the pack (if not empty) to the neighbour's
   queue — left for packs 1 and 3, right for pack 2, the same rotation
   `DraftState::rotate_packs` performs. A round is over when every pack
   of the round is empty; the next round's packs are then dealt to every
   seat at once. A seat can have several packs waiting (it is slower than
   its upstream neighbour); it sees how many. This is `mtg-draft/src/
   table.rs`, a new state machine beside `DraftState`, with its own unit
   tests (direction per round, a two-pack queue, round completion, the
   pick numbering a pack carries: pick `pack_size - remaining + 1`).
   AI seats pick on their own worker thread whenever their queue is
   non-empty, one pick at a time per seat, through
   `DraftLlmClient::build_pick_prompt` / `send_pick_message` — the same
   prompt the runner sends, with the same `Table` facts.
3. **Deck building.** Starts for a seat as soon as its draft is over (its
   last pack of round 3 is picked) — it does not wait for the table. A
   human picks any cards of their pool into a main deck and sets basic
   land counts; the server validates with `mtg_draft::deckbuilding::
   validate_deck` (40 minimum, only pool cards, basics unlimited) and
   refuses an invalid deck with the reason, the way the game page is
   refused an unreadable answer. An AI seat builds through the moved
   `build_deck_with_llm`, with the runner's fallback on failure. Every
   deck is written to `<log dir>/decks/seat-N.txt` in the runner's
   `COUNT NAME` format, so it can be played by `mtg-runner --deck1` later.
4. **Playing.** When every deck is in, a `Tournament` is created
   (`--best-of`, rounds as the runner computes them) and each round's
   pairings are played in parallel threads, as the runner does. A human
   seat's game is a `GuiPlayer` bound on the server's `--bind` address on
   the next port from `--game-ports` (default `8801-8899`); the page shows
   the human "Round 1 vs seat 3: open http://host:8803/" and links it. A
   `cli` seat plays in the terminal. An AI seat is `make_game_player`
   (moved to the lib). A human-vs-human match is two `GuiPlayer`s on two
   ports. Results are recorded, standings shown, next round paired when
   the round is over. Each game's log goes to the draft log as the
   runner's does. `GuiPlayer::new` needs a bind-address parameter for this
   (`mtg-runner` keeps `127.0.0.1`).
5. **Done.** Final standings on the page and in the log; the server stays
   up until the host quits so the page can be read.

### Timers and absent humans

`--pick-seconds N` (default none) auto-picks for a human seat that has
not picked in N seconds: the pick is `mtg_draft::deckbuilding::fallback_
deck`'s notion of the best card if one exists, otherwise the first card,
and it is logged as an auto-pick and shown to the seat. `--build-seconds`
does the same for deck building with `fallback_deck`. Without timers the
table waits; the host can always `kick <seat>` from the terminal to turn
a human seat into an auto-picking one for the rest of the run.

### Protocol

WebSocket JSON on `/ws?seat=N&key=K`, text frames, one object per frame,
as the game page does. The server sends the seat's **whole view** after
every change (reconnect-safe, no deltas). The page never sees another
seat's pack or pool contents — only public facts (names, status, pick
counts, standings, the cards other seats have already taken are NOT
shown, as in a real draft).

Server → client:

```jsonc
{
  "type": "view",
  "phase": "lobby" | "drafting" | "building" | "playing" | "done",
  "seat": 0,
  "pod_size": 4,
  "set": "isd",
  "seats": [ {"seat": 0, "kind": "human"|"ai"|"cli", "name": "seat 0",
              "joined": true, "status": "picking"|"waiting"|"building"|"ready"|"playing"|"idle",
              "picks": 12} ],
  "pass_direction": "left"|"right"|null,     // for the round in progress
  "pack": {                                   // null when nothing to pick
    "id": 7, "round": 1, "pick": 3, "size": 14,
    "cards": [ {"index": 0, "name": "Chapel Geist", "line": "Chapel Geist {1}{W}{W} 2/3 Creature — Spirit | Flying",
                "text": "Flying", "rarity": "common", "colors": ["W"]} ],
    "waiting": 1,                             // packs queued behind this one
    "deadline_ms": null                       // or ms left on --pick-seconds
  },
  "pool": [ {"name": "...", "line": "...", "text": "...", "colors": [...], "rarity": "..."} ],
  "picks": [ {"round": 1, "pick": 1, "card": "...", "auto": false} ],
  "deck": null | {"main": ["name", ...], "lands": {"Plains": 7, "Island": 10}, "sideboard": [...], "valid": true, "problem": null},
  "matches": [ {"round": 1, "opponent": 3, "url": "http://192.168.1.20:8803/", "status": "waiting"|"playing"|"done",
                "games": [ {"winner": 0} ], "result": "2-1"} ],
  "standings": [ {"seat": 0, "wins": 2, "losses": 0, "points": 6} ],
  "notice": null | "text the server wants this seat to read once"
}
{ "type": "refused", "reason": "Card 'X' is not in this pack", "echo": {...the request...} }
```

Client → server:

```jsonc
{ "type": "pick", "pack_id": 7, "index": 3 }          // index into pack.cards
{ "type": "deck", "main": ["..."], "lands": {"Plains": 7}, "sideboard": ["..."] }   // can be re-sent until "ready"
{ "type": "ready" }                                    // deck final; the seat is ready to play
{ "type": "name", "name": "Lawson" }                   // optional, cosmetic
```

The server also serves `GET /` → `mtg-gui/draft.html`, `GET /dist/*`,
`GET /assets/*` from the same `mtg-gui` directory as the game page
(`$MTG_GUI_DIR` or `./mtg-gui`), and `GET /api/view?seat=N&key=K` → the
same view JSON (what the terminal client and the tests read).

### The page

`mtg-gui/draft.html` + `src/draft/*.ts` → `dist/draft/*.js`, compiled by
the same `tsc -p .` (a second entry point; `dist/` stays committed). It
is a DOM page, not the 640x360 canvas: a draft is lists of cards with
rules text, and the game page's pixel-art frame is the wrong tool for a
45-card pool. Art from `assets/art/` where a card has it, the game page's
fonts, and the same dark palette, so it reads as one program.

Views, by phase:

- Lobby: who is here, who is not, the set, pod size, pass direction.
- Drafting: the pack as a grid (art, name, cost, type, P/T, rules text on
  hover and in a side panel), click to select, click again or press Enter
  to pick; "pick 3 of 14, 1 pack waiting"; the pool on the right grouped
  by colour with counts; the timer if any. Keyboard: digits/arrows to
  move, Enter to pick, as the game page's keys.
- Building: the pool as a checklist (click to move a card between main
  and sideboard), land steppers for each basic, a running count
  (`23 spells + 17 lands = 40`), a colour/curve summary, and a "Ready"
  button that is disabled with the reason until the deck validates.
- Playing: this seat's matches with the game link, live results as they
  come, standings; a "waiting for the others" line.
- Done: standings.

Everything is sized in rendered lines: a 14-card pack fits one screen at
1280x720 and at phone width scrolls, never clips (CLAUDE.md's rule).
`window.mtgDraft` exposes the last view and `window.mtgDraftDebug.stage(
view)` renders a view without a socket, so a test or a person can stage
any state, as `window.mtgDebug.stage` does on the game page.

### The terminal client

`mtg-draft-client ws://host:8800/ws?seat=N&key=K`: a plain line-oriented
client, not a TUI. It prints the view when it changes (the pack as a
numbered list with the runner's `pack_line`, the pool as `pool_listing`),
reads a number to pick, and in deck building takes `add <n>`, `drop <n>`,
`lands plains=7 island=10`, `show`, `ready`. It prints the game URL when
a match is paired. It is also what the integration tests drive.

### What the AI seats are told

Exactly what the runner tells them: the system prompt with the set's
card reference and the draft rules, `build_pick_prompt` for each pick
with the seat's place at the table, the deck-building prompt, and in
games the `LlmPlayer` system prompt with their decklist. Nothing about
which seats are human. The `--guide` flag is passed through.

### Logs and files

`--log <path>` writes the runner's draft log format through `DraftLogger`
(header, packs, every pick with its prompt and response for AI seats and
`human` for humans, pools, deck building, matches, standings), so the
drafting playtest guide's readers apply. The directory beside it holds
`decks/seat-N.txt` and `games/r<round>-<a>v<b>-g<n>.log`. No snapshot or
resume in this version; the log says so if `--resume` is passed.

## Tests

- `mtg-draft/src/table.rs`: the asynchronous table (unit tests above).
- `mtg-draft-runner/tests/lobby_*.rs`: the server as a subprocess on a
  free port with `--seats human,human,ai,ai` under a `CLAUDE_CODE_BIN`
  stub (see `mtg-draft-runner/tests/one_claude_code_driver.rs` for the
  stub shape), two `tungstenite` clients that join, pick to the end,
  build decks and send `ready`; assert the picks the view shows are the
  ones the log records, that no view ever carries another seat's pack or
  pool, that the match URL appears and a game between the two stub AI
  seats completes, that a reconnecting client is sent the same pending
  pack, that an invalid deck is refused with a reason, that a second
  client with a wrong key is refused, and that `--pick-seconds 1` picks
  for an absent human. One test drives the terminal client through a
  pipe instead of a socket.
- `mtg-gui/tests/draft_page.js` (Playwright): stage each phase's view,
  screenshot, click a card and read the `pick` the page sent, build a
  deck through the checklist and read the `deck` and `ready` messages;
  added to `.github/workflows/gui.yml`, with the `dist/` freshness check
  covering `dist/draft/`.
- `mtg-player/tests/gui_protocol.rs`-style check that the view JSON the
  server writes is one the page's `protocol` types name (a fixture file
  written by a Rust test and read by a node test).

## Playtest before calling it done

Host a 4-seat table (1 human driven by Playwright, 3 stub AI), draft to
the end, build, play one round through the game page with autoplay
clicks, and read the log and the screenshots. Then one 2-human table
(two Playwright pages) to see the human-vs-human match get two ports.
Then one table with a real `ai:cc:claude-sonnet-4-6` seat for the picks
only, to confirm the LLM path is wired. Every finding is fixed or filed.

## Out of scope, said so

Snapshot and resume of a lobby; spectators; a draft of more than one set
or a cube; seat-to-seat chat; authentication beyond the key.
