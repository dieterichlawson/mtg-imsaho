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
passes to your next precombat main phase (and stops for any prompt, a
spell on the stack, or that main phase), `l` opens the log, `g`/`G` a
graveyard, `e` exile, `d` your library, `s` stops at every priority
instead of passing when there is nothing to do.

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

All four run in CI (`.github/workflows/gui.yml`), which also checks
that `dist/` matches `src/`.
