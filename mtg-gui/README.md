# mtg-gui — the pixel-art page

The fourth surface for a decision, next to the CLI, the LLM prompt and
the random seat. The runner serves this directory on a local port and
plays one seat through a browser.

```bash
cargo build --release
./target/release/mtg-runner --p1 gui --p2 random          # then open http://127.0.0.1:8765/
./target/release/mtg-runner --p1 gui:9000 --p2 claude-code # a port of your choosing
./target/release/mtg-runner --p1 gui:8765 --p2 gui:8766    # two humans, two tabs
```

Run from the repository root: the page is read from `./mtg-gui` (or
`$MTG_GUI_DIR`), the way `data/` and `decks/` are. `--save`, `--resume`,
`--log` and `--seed` work as for any seat. Closing the tab and reopening
it is fine: a reconnecting page is sent the decision still pending.

## How it works

The seat (`mtg-player/src/gui.rs`) sends each decision as the engine's
own `GameView` and `LegalActions` serialized to JSON, and takes one
`Action` back. Nothing is described twice. The page is TypeScript in
`src/`, compiled by `tsc` to `dist/`, which is committed so the runner
serves it from a bare checkout with no node toolchain:

```bash
cd mtg-gui && npm run build     # or: tsc -p .   (typescript 5+)
```

- `src/protocol.ts` — the engine's types as serde writes them.
- `src/state.ts` — the page state and the widget shape.
- `src/main.ts` — the socket, the state, mouse and keys.
- `src/prompts.ts` — a decision to a widget: menu, pick, mark, attackers,
  blockers, list, order, number. An unknown prompt kind is a list of the
  offered actions, never nothing.
- `src/render.ts` — the 640x360 frame, integer-scaled, and the list of
  what was drawn where (input looks at the last frame, not the view).
- `src/assets.ts` — art and fonts; a card with no art file gets a
  coloured placeholder.

Edit `src/`, run the build, commit both.

Playing: click a card to see what it can do, then click the verb;
double-click a card with one verb to do it. A cast that needs targets
highlights them; click one. Sets are marked and confirmed. Attackers are
clicked on (again to change whom they attack, again to withdraw);
blockers are clicked, then the attacker they block.

Keys: Enter passes or confirms, Esc or right-click backs out, `f`
passes to your next precombat main phase — this turn's, if pressed at
your upkeep or draw step (and stops for any prompt, a spell on the
stack, a land you can play, the opponent's attack on you, your own
postcombat main phase, a spell or ability on offer on your own turn once
that turn is reached, or that main phase — the CLI's stops; it will not
start where it would stop at once, and says what it passed up), `l`
opens the log, `g`/`G` a
graveyard, `e` exile, `d` your library, `s` stops at every priority
instead of passing when there is nothing to do.

## The draft page

`draft.html` + `src/draft/*.ts` → `dist/draft/*.js`, the second entry
point of the same build, served by `mtg-draft-server` at `GET /` (see
`docs/plans/draft-with-friends.md`). A DOM page, not the canvas: a draft
is lists of cards with rules text. Open the join line the server prints
(`http://host:8800/?seat=0&key=...`); the page takes `seat` and `key`
from its own query string and opens `/ws?seat=N&key=K`.

The server sends the seat's whole view after every change and the page
is rebuilt from each one, so closing the tab and reopening it is fine
and a dropped socket reconnects with backoff. A request the server
refuses (`{"type":"refused", "reason", "echo"}`) is shown with its
reason, and the page goes back to what the last view describes.

- `src/draft/protocol.ts` — the view, the refusal and the four messages
  the page sends (`pick`, `deck`, `ready`, `name`), as types and as a
  shape table the fixture test reads.
- `src/draft/cards.ts` — the headline's parts (cost, type, P/T), mana
  value, colour grouping, art lookup by name.
- `src/draft/deck.ts` — the deck under construction, its message, the
  reason it does not validate yet (40 cards; the server checks again),
  the colour and curve summary.
- `src/draft/render.ts` — one function per phase: lobby, the pack grid
  with the rules text in a side panel, the pool checklist with land
  steppers and the Ready button, matches and standings.
- `src/draft/main.ts` — the socket, the state, the keys, and the hooks.

Drafting: click a card to read it, click it again or press Enter to
pick it; digits and arrows move the selection, Esc clears it. Building:
click a card to move it between main and side, step the basics, and
press Ready when the count line reaches 40 (the button says why it is
disabled until then). Playing: the game link is a link to the game page.

`window.mtgDraft` holds the state, the last view and every message the
page sent (`.sent`); `window.mtgDraftDebug.stage(view)` renders a view
without a socket and `stage({type: "refused", ...})` applies a refusal,
which is how the tests and a person stage any state.

Tests, both in CI:

- `node mtg-gui/tests/draft_fixtures.js` — every view in
  `tests/draft-page-fixtures.json` (the page's own, one per phase) and
  in `tests/draft-view-fixtures.json` (the server's, when its test has
  written it) has every field `protocol.ts` declares. No browser.
- `NODE_PATH=$(npm root -g) node mtg-gui/tests/draft_page.js --shots DIR`
  — the page served by the test itself, every phase staged at 1280x720
  and at 390px wide (a 14-card pack fits one screen; a phone scrolls
  down, never sideways), a pick clicked and keyed, a deck built through
  the checklist, a refusal shown, the reconnect backoff seen.

## Art

`assets/art/` holds one 64x48 image per card face and token, drawn by
Retro Diffusion from the card's own data with the printed art as a
contrast-lifted image-to-image source, plus UI pieces. `manifest.json`
records prompt, style, size, seed and cost per image. To regenerate one
or all:

```bash
RETRO_DIFFUSION_API_KEY=rdpk-... scripts/gen_card_art.py cards --only "Abattoir Ghoul" --force
RETRO_DIFFUSION_API_KEY=rdpk-... scripts/gen_card_art.py cards tokens ui --dry-run
```

Fonts: Silkscreen and Press Start 2P, both under the SIL Open Font
License (`assets/fonts/OFL-*.txt`).

## Tests

- `cargo test -p mtg-player --test gui_protocol` — every decision point
  of seeded random games serializes, and every prompt kind the engine
  defines is one `prompts.ts` names.
- `NODE_PATH=$(npm root -g) node mtg-gui/tests/smoke.js --shots /tmp/shots`
  — a real runner and the Playwright Chromium: keep, play a land through
  the popover, pass, no page errors. Needs `cargo build -p mtg-runner`
  and the `playwright` package with its Chromium.
- `node mtg-gui/tests/xfunding.js` — the page's X-funding allocator
  against the engine's own answers (`tests/x-funding-cases.jsonl`, written
  by `mtg-player/tests/gui_protocol.rs`). No browser, no runner.
- `NODE_PATH=$(npm root -g) node mtg-gui/tests/widgets.js` — one
  synthetic decision per prompt kind over a real board: the widget the
  page chooses, the clicks that answer it, and the Action it sends.
- `NODE_PATH=$(npm root -g) node mtg-gui/tests/autoplay.js --games 4`
  — whole games against the random seat, played through the page by
  clicking at random; fails on a decision the page offers no way to
  answer, a page error, or an invariant violation. `--shots DIR` keeps
  a screenshot every 25 decisions.

All four run in CI (`.github/workflows/gui.yml`), with the draft page's
two above, and CI also checks that `dist/` (both entry points) matches
`src/`.
