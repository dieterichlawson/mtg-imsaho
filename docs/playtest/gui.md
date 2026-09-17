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

- G12 [proposed 2026-09-17, from tonight's G1 night and #523] auto-pass's
  silence: `main.ts:96` answers every pass-only priority without drawing a
  frame, so a run of them is time the player never sees. Instrument
  `window.mtgDebug.trace`, count consecutive `only pass` runs across a whole
  game, and for the longest run ask what the player is told happened in it.
  Then press `s` (stop at every priority) and confirm the page stops, says so,
  and that `f` and `s` compose sanely rather than fighting.

- G13 [proposed 2026-09-17, observed during #517's repro but not filed] the
  dedup'd duplicate: the engine offers `PlayLand` for one object id even with
  four Forests in hand, so three visually identical hand cards carry no verb
  marker and answer no click — clicking one only moves the inspector. Sweep
  every prompt where the engine dedups by name (the land drop, a second copy
  of the same spell) and decide what the page owes the copies it draws but
  cannot act on. #512 is the CLI's version of the same question.

- G14 [proposed 2026-09-17, from #522] an unclipped-text sweep: #522 was found
  by eye, and it is the *one* string on the board drawn without `clip()` only
  as far as anybody has looked. Enumerate every `text()` call in `render.ts`
  not wrapped in `clip()` or `wrap()`, and drive each against the longest
  string it can actually hold — the longest card name in the pool, the longest
  step name, a full keyword list, an eight-line oracle text, a player label
  after #519. Measure with `ctx.measureText` against the pane width rather
  than by looking.

- G15 [proposed 2026-09-17, from the G5 runs — unverified] the window that is
  too small: `index.html` sets `body { overflow: hidden }` and `fitCanvas`
  floors the integer scale at 1, so a window narrower than 640x360 should crop
  the canvas with no scrollbar and no way to reach the right-hand panel — where
  every button and the whole prompt live. Check a phone-sized window, a
  half-screen window and a very wide short one.

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
  [2026-09-17] The first half holds: engaged at your own precombat main,
  5/5 across seeds 11/5/23 skipped the rest of the phase and stopped at the
  declare-attackers prompt with an accurate notice ("Auto-pass off: you are
  asked something"). **The other half is still unprobed**, and the reason is
  worth knowing before you try: engaged at your own main phase, `f` can never
  reach the thing it is documented to reach, because your own declare-attackers
  prompt is a non-menu prompt and stops it first, every turn. To test "passes
  to your next precombat main phase" at all you have to engage it during the
  OPPONENT's turn. Do that, and check what `f` does when a spell goes on the
  stack in a window where you could actually respond — `main.ts:96` auto-passes
  pass-only priorities before `autoPassDecides` is ever consulted, so the
  "stops for a spell on the stack" clause may be unreachable by construction.
- G9 two tabs on one seat, and a tab closed mid-prompt and reopened: the
  pending decision must come back, and an answer from the stale tab
  must not land on a newer prompt.
  [2026-09-17] The rule itself HOLDS under three attacks — a stale `seq` on a
  live socket, two tabs racing the same live `seq`, and a stale tab clicking
  after the seat moved on; exactly one answer landed every time, and a genuine
  close-and-reopen gets the pending decision back. What broke was everything
  around the drop (#515, #516), so re-probe the surroundings, not the rule.

- G16 [proposed 2026-09-17, from #524, #520 and #518] the destructive default:
  for every widget, ask what Enter does when the player has touched nothing.
  `beginMark` at `min 0` commits an empty set on the spot (#520) and refuses
  below the minimum without a word (#518, #524). Not yet asked of the others:
  `beginNumber` with an empty field (`Number("")` is `0`, which passes both
  `Number.isInteger` and the range check, so Enter looks like it submits X=0),
  `beginOrder` without reordering, `beginPick`'s Decline. Compare each against
  the CLI's same prompt, which refuses out loud in both directions.

- G17 [proposed 2026-09-17, from reading `mtg-player/src/gui.rs:190` during G9]
  the seat that can never give up: `ask`'s
  `let Ok(answer) = self.answers.recv() else { return Action::AbandonGame }`
  looks unreachable — `Shared` holds a clone of the `Sender` for the life of
  the `GuiPlayer`, so the receiver never disconnects. Start
  `--p1 gui:PORT --p2 random`, answer one prompt, then close the browser
  entirely. Verify whether the runner blocks for ever, whether `--max-actions`
  and the progress watchdog can fire while `ask` blocks (neither runs), and
  whether the "waiting for a browser at …" line is ever re-printed
  (`said_waiting` latches, and `Shared::connected()` counts channels reaped
  only on the next broadcast). Then decide whether a human seat *should* wait
  for ever, and if so fix the comment rather than the code.

- G18 [proposed 2026-09-17, from G9] a notice belongs to one tab but is sent to
  all: `GuiPlayer::notice` broadcasts, while `main.ts`'s notice branch only
  restores the decision when `lastSent.seq === msg.seq`. With two tabs open,
  make tab A send `window.mtgSend({Nope: 1})` and read what tab B shows. Also
  check the `seq: 0` "unreadable message" notice, which no tab can ever match.

- G19 [proposed 2026-09-17, from G9] the typed field after a refusal:
  `main.ts`'s notice branch calls `beginDecision` but not `syncField`, and
  `send()` already called `hideField()`. At a `ChooseXFunding` (`number`) or a
  filtered `list` prompt, send `window.mtgSend("AbandonGame")` and check
  whether the DOM input comes back before the next canvas click.

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
