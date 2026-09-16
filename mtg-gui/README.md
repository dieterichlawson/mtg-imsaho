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
`Action` back. Nothing is described twice. The page:

- `src/main.js` — the socket, the state, mouse and keys.
- `src/prompts.js` — a decision to a widget: menu, pick, mark, attackers,
  blockers, list, order, number. An unknown prompt kind is a list of the
  offered actions, never nothing.
- `src/render.js` — the 640x360 frame, integer-scaled, and the list of
  what was drawn where (input looks at the last frame, not the view).
- `src/assets.js` — art and fonts; a card with no art file gets a
  coloured placeholder.

Keys: Enter passes or confirms, Esc cancels, `l` opens the log, `g`/`G`
a graveyard, `e` exile, `d` your library, `s` stops at every priority
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
  defines is one `prompts.js` names.
- `NODE_PATH=$(npm root -g) node mtg-gui/tests/smoke.js --shots /tmp/shots`
  — a real runner and the Playwright Chromium: keep, play a land through
  the popover, pass, no page errors. Needs `cargo build -p mtg-runner`
  and the `playwright` package with its Chromium.
