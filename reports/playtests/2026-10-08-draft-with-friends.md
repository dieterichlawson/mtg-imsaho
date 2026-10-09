# Draft with friends — the first playtest (2026-10-08)

The hosted draft (`mtg-draft-server`, the draft page, the terminal
client) was built by two agents who never ran it together. This is the
"Playtest before calling it done" section of
`docs/plans/draft-with-friends.md`, run as the people at the table would:
a person in a browser driven by Playwright, people at two browsers, a
person at the terminal client through pipes, the edge list, and one table
of real `claude` seats. Every finding below is fixed in a commit on this
branch or filed under "Known gaps after the first playtest" in the plan.

Screenshots and run output: `logs/draft-playtest/` (gitignored; the
directory names below are under it). Ledger rows: D27–D32 in
`reports/playtests/LEDGER.md`.

## What was run

| run | table | how | result |
|---|---|---|---|
| `page/` | none | `node mtg-gui/tests/draft_page.js --shots` | the staged page at 1280x720 and 390px, before and after the fixes |
| `t1/` | `--seats human,ai,ai,ai --best-of 3 --seed 101` | one Playwright person: 42 picks by clicking (a screenshot every 5), a 40-card deck through the checklist, Ready, both rounds through the game page with random clicks (`scratchpad/table.js`, the same step logic as `autoplay.js`) | the tournament ran to its end: seat 0 went 1-1; 11 game logs under `games/`, four `decks/seat-N.txt` |
| `t2/` | `--seats human,human,ai,ai --best-of 3 --bind 0.0.0.0 --seed 202` | two Playwright people; seat 1 concedes after 25 decisions, seat 0 after 80 | the people's match was two pages on two ports (8821, 8822; round 2 on 8823/8824), round 2 paired on the result (0 v 3, 1 v 2), standings 3 / 0 / 2 / 1 |
| `t4/` | `--seats human,ai --best-of 1 --seed 404` | `mtg-draft-client` through pipes (`scratchpad/client_probe.py`): 42 picks by number, `show`/`help`/a bad word/`ready` out of phase, a 39-card deck, `add 99`, `drop`/`add`, `ready`, then the match through its page | the whole draft and build worked; transcript `client-transcript.txt` (4,620 lines) |
| `t5-instant/` | `--seats human,ai --pick-seconds 4 --best-of 1` with the host pressing Enter before anybody joined | the late-join probe (`scratchpad/edges.js`) | **the draft was over before the person arrived** (finding 5) |
| `t5/` | the same, on the fixed server | late join + second tab + wrong key + seat 9 + AI seat + join after the end + a 39-card deck + `kick` mid-match + `quit` | see the edge list below |
| `t6/` | `--seats human,ai,ai,ai --pick-seconds 1`, Enter, SIGINT 1.2 s in | `scratchpad/ctrlc.sh` | gone in 2 s, no stub children, no ports held; nothing in the log (gap 1) |
| `t7-real/` | `--seats human,ai:cc:claude-sonnet-4-6 x3 --pick-seconds 1`, the person absent | the real `claude` CLI | 27 real picks with reasoning ("Bloodgift Demon ... a 5/4 flyer that draws a card per turn") before the session quota ran out; each seat gave up after its 600 s budget and the table took it over, as designed |

All AI seats except t7's are the stub `claude` from
`mtg-draft-runner/tests/lobby_support/mod.rs` (`CLAUDE_CODE_BIN`), which
picks card 0, builds 23 over 9 Island 8 Swamp, and fills a game schema.

The sandbox this ran in proxies every non-loopback TCP connection and
answers a WebSocket upgrade with a 502, so for t2 the pages were opened
on `127.0.0.1` and the advertised `192.0.2.2:88xx` game links were
rewritten to loopback by the driver; the links themselves were checked
as printed (`192.0.2.2` is this machine's non-loopback address, found
by `advertised_host`).

## Findings

Fixed, each with its commit and a test that would have caught it:

1. **The page could not read what the server sends.** `matches[].url`
   was declared a string and `games[].winner` a number; the server sends
   `null` for a match whose pages are not open yet, a forfeit, the
   opponent's side of a match, and a drawn game. The server's fixture
   failed the page's shape check, and a person would have read "Open your
   game: null". The page also dropped the fields the server sends beyond
   the plan: `pairings`, `seats[].connected`, `seats[].auto`,
   `deck.ready`, `build_deadline_ms` — so the whole round, an away seat,
   a seat the table picks for and the build timer were invisible.
   Commit `9397b50`; `draft_page.js` stages `playing_unlinked`;
   `draft_fixtures.js` is green on both fixture files.
   Screens: `page/playing_unlinked-1280.png`, `t2/t2-s0-25-match-r1-paired.png`.

2. **Game 2 of a match started under game 1's GAME OVER box.** A hosted
   match plays through one `GuiPlayer`, and the page never cleared
   `gameOver`: the box and the panel's "GAME OVER! SEAT 1 WINS" stayed
   over game 2's board until a click, and the driver logged game 2's
   every decision as game 1 ending again (sixty lines in t1 round 2).
   Commit `071933f`; `mtg-gui/tests/next_game.js` (in the gui workflow).
   Screen: `t1/t1-s0-game-r1-resumed-05-next-game-under-gameover.png`.

3. **Three of four decks were above the "DECK BUILDING" header** in the
   log: the section opened when the whole draft was over, but a seat
   builds when its own is. Commit `7230a5c`; `lobby_two_humans_two_ai`
   asserts the first POOL and DECK records are below the header.

4. **Every card added was a refusal.** The page and the client send the
   deck after every card moved, and the server refused each one until
   the fortieth: 22 "refused: Deck has N cards" lines at the terminal
   (`t4/client-transcript.txt` lines 3139–3555), a red banner per click
   on the page. A short deck is now recorded with its problem (which the
   view carried already) and refused only at `ready`; a card not drafted,
   too many copies or a non-basic land is still refused. Commit `8c48ad4`;
   `draft_view_fixtures`, `lobby_two_humans_two_ai`, a `lobby` unit test.

5. **An absent seat lost the whole draft in four seconds.** With
   `--pick-seconds 4` and the host pressing Enter, the table picked for
   the absent seat the instant each pack landed; the stub seats pass at
   once, so the person who joined fifteen seconds later found the draft
   over, a deck built and the match forfeited (`t5-instant/`). The plan's
   "auto-picked until it joins; see timers" meant the timer's pace: an
   absent seat is now picked for when its timer runs out (`t5/draft.log`:
   picks exactly 4 s apart, `the seat is away`), joining hands the seat
   back with the timer running, no timer and a kicked seat pick at once.
   Also found on the way: `Lobby::new` accepted packs dealt for a
   different number of seats than `--seats` (the pass went to a seat
   nobody sat at), and a kicked seat's "handed to the table" notice was
   overwritten by the table's first pick. Commit `ba7c52c`;
   `an_absent_seat_under_a_timer_is_picked_for_when_the_timer_runs_out`,
   `a_table_dealt_for_the_wrong_number_of_seats_is_refused`.
   Screens: `t5/timer-joined.png`, `t5/timer-timer-picked.png`.

6. **A wrong link retried for ever.** A wrong key, `?seat=9` or an AI
   seat's number got the right banner ("Refused: wrong key for seat 0")
   and under it "disconnected — retrying in 1s": five reconnects in
   2.5 s, each refused the same way. Commit `eaba840`; `draft_page.js`.
   Screens: `t5/refusals-wrong-key.png`, `t5/refusals-bad-seat.png`.

7. **The terminal client offered a link to a finished match**
   ("Round 1 vs seat 1: done — open http://127.0.0.1:8831/ — 0-1").
   Commit `58427ab`; a unit test in the client.

8. **`quit` left no word in the log**: it ended in a usage summary.
   Commit `9b9935c`; the absent-human lobby test types `quit`.

Filed as gaps in the plan (too large, or not this feature's):

- **Ctrl-C of the server** exits by the default signal action: within
  2 s, no stub children left, ports freed — but the log ends mid-pick
  with no NOTE or FATAL, and nothing sweeps a real `claude -p` child
  that is mid-call.
- **`kick` during a match in progress** does not end the match: the
  page waits for a browser nobody will open (`t5/server.err`: "kicked ...
  its games are forfeit" then nothing), and `quit` is the host's only
  way out. The guide says "kick before the round"; a kicked seat's
  `GuiPlayer` should give the game up.
- **A target pick with nothing to click** on the game page: "Cobbled
  Wings: choose a target — click a highlighted card or player" with no
  highlighted card and only Cancel (t1 round 1 game 1, decision 74,
  `t1/t1-s0-game-r1-05-stuck-74.png`, `t1/games/r1-0v1-g1.log`). A
  person has Cancel; the random driver did not know to press it. Not a
  draft seam; the game page's.

Checked and correct (no finding):

- The draft page at a 4-seat table: pick line, "N packs waiting", the
  side panel reading the selected card, the pool grouped by colour, the
  checklist with the running count, the Ready reason at 39 cards, the
  countdown; no horizontal overflow at 1280 or 390.
- The picks the view shows are the ones the log records, with `human`
  in the prompt and response slots; `[Seat 0] POOL (42 cards)`; the
  deck file `decks/seat-0.txt` sums to 40 in `COUNT NAME` lines.
- A second tab with the same key sees the same seat and pack, the seat
  counts two connections, a pick in one tab is the seat's in the other,
  closing a tab drops the count.
- A join after the end sees the done page with the final standings,
  every pairing and the deck.
- The people's match is two pages on two distinct ports; the AI-vs-AI
  match plays on its own; round 2 pairs winners with winners; the
  standings order by match points then game wins.
- The terminal client: a numbered pack, the pool with counts, `add`/
  `drop`/`lands`/`ready`, `add 99` refused by the client, `ready` out
  of phase refused by the server, a number with no pack answered.
- What the AI seats are told at a mixed table is exactly the runner's
  prompt ("Pack 1 of 3, Pick 1 of 14. You are seat 1 of 4; after your
  pick this pack passes LEFT to seat 2") with nothing about which seats
  are people; a seat whose CLI gives up is handed to the table and its
  games forfeit, and the standings say so.

Not confirmed in this run: a deck build through the real `claude` CLI
(the session quota ran out at pick 9); the build path is the runner's
`build_deck_with_llm`, which the stub exercised at every table.

## Commits

- `9397b50` gui: the draft page reads what the server really sends
- `071933f` gui: the next game of a match comes down on the page without the last game's box over it
- `7230a5c` draft server: the log's DECK BUILDING section opens with the first finished seat
- `58427ab` draft client: a finished match is printed as its result, not as a page to open
- `eaba840` gui: the draft page stops reconnecting when the server refused the join itself
- `ba7c52c` draft server: an absent seat under a pick timer is picked for on the timer, not at once
- `8c48ad4` draft server: a deck that is only short is work in progress, not a refusal
- `9b9935c` draft server: the log says the host quit, and where the table was
