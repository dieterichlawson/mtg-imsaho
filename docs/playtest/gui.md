# Playtesting the GUI

Subject: the browser page — `mtg-runner --p1 gui` and `mtg-gui/`, the
fourth surface a decision is presented on. Not the rules, and not the
engine's prompts as such (those are the game's subject): whether a
person at the page can see what is going on, find what they can do, do
it, and never be left with a question they cannot answer.

## Before you start

Read `mtg-gui/README.md` and the three tests that already pin
contracts: `mtg-player/tests/gui_protocol.rs` (every prompt kind the
engine defines is one the page names), `mtg-gui/tests/widgets.js`
(every widget answers), `mtg-gui/tests/autoplay.js` (whole games through
the page). What a night can do that they cannot is *look*: at the
screenshots, at a board of thirty permanents, at a prompt no seeded
random game happened to reach.

## Where to look

- `mtg-gui/src/prompts.ts` is the decision-to-widget mapping. A prompt
  kind that lands in its `default` arm is a plain list, which is safe
  but may be a bad way to ask that question.
- `mtg-gui/src/render.ts` is the frame: every rectangle it draws and
  what a click on it does. `mtg-gui/src/main.ts` is input and the socket.
- `mtg-player/src/gui.rs` is the seat: it sends `GameView` and
  `LegalActions` as they are and takes one `Action` back.
- Drive it with Playwright: `mtg-gui/tests/autoplay.js --shots DIR`
  keeps a screenshot every 25 decisions, and the page exposes
  `window.mtg` (state), `window.mtgSend(action)` and
  `window.mtgDebug.stage(decision)` for staging a prompt over a real
  board without a socket.

## Ideas

**The Stranger** has never seen the CLI and does not know the engine.

- G1 sit down cold: from the URL, can you keep a hand, play a land,
  cast a creature, attack and block without reading the README? Note
  every moment you did not know what to click.
- G2 what just happened: the opponent cast something and it resolved
  while you had nothing to respond with. Does the board tell you what
  changed? (The band shows the last two log lines; is that enough?)
- G3 the prompt with no board: a search, a looked-at pile, a card name.
  Is the row list readable at 30 entries? Does the filter box take
  focus, and give it back?
- G4 the crowded board: `20 Armored Skaab` a side, a 40-card graveyard,
  nine tokens. Does every row fit its band, does the graveyard page, can
  you reach the last card? (The CLI's V7 sweep, for the page.)
  [2026-09-17] Both halves answered, opposite ways. The **zone overlay
  holds**: 137 cards page 50/50/37, nothing off-pane, every card reachable
  including the last, and the page index is rebuilt per open — don't re-probe
  it without a reason. The **battlefield row does not** (#513): 20 is far too
  few to find anything, because `rowLayout`'s stride floor is only reached at
  43 and the row first crosses the pane at 45 (the hand at 42). Reach it with
  tokens — `4 Army of the Damned / 4 Endless Ranks of the Dead / 30 Swamp`
  against `40 Plains` crosses 45 by turn 18 and hit 108 on seed 11. The
  measurement to copy: don't eyeball the screenshot, read `window.mtg.hits`
  after `window.mtgDebug.render()` — every card's drawn rectangle is in there,
  so "is it off the pane" is `h.x + h.w > 480` and "is it invisible" is
  `h.x >= 480`, both exact.
- G5 game over: a win, a loss, a draw, a concede, a decked opponent.
  Is the reason on screen? Is the last board still readable behind it?

**The Fumbler** clicks the wrong things.

- G6 click everything that is not highlighted during a pick: the
  opponent's life, a stack item, a land, the log. Nothing should answer
  the prompt by accident, and Escape and right-click should always back
  out of a cast that has not been paid for.
- G7 double-click, right-click, wheel, Enter and Escape at every widget,
  in the wrong order. Enter on a mark with too few marked must refuse
  out loud; Enter on a menu passes and must be the only key that does.
- G8 `f` at the wrong moments: engaged at your own main phase (it skips
  it, and says so), during combat, with a spell on the stack.
- G9 two tabs on one seat, and a tab closed mid-prompt and reopened: the
  pending decision must come back, and an answer from the stale tab
  must not land on a newer prompt.

**The Reader** cares about what the inspector says.

- G10 hover every kind of thing: a token, a transformed card, a copy, an
  equipped creature, a cursed player, a stack ability, a card in exile.
  Everything the CLI's `i` screen shows should be here; #333, #468,
  #501, #504 list what has been missing before.
- G11 art and names: a card whose art file is missing renders a
  placeholder, never blank; a stack ability shows its source's art.

## Filing

As `docs/playtest/README.md` says, with `surface:gui` in the title. A
finding on this surface is usually a question about the other three:
if the page cannot ask something well, check whether the CLI and the
LLM prompt can before writing it up as the page's problem alone.
