# A pixel-art GUI for the game

Status: phases 1 to 4 built on the recommendations below (the browser
page served by the runner, TypeScript compiled by `tsc` to a committed
`dist/`, 640x360, one GUI seat per port so two humans are two tabs, art
generated at 64x48 with `rd_plus__low_res` from the printed art at
strength 0.78).
See `mtg-gui/README.md` for running it. Each section below is one
decision, with what the code already fixes, the options, and the
recommendation that was taken.

## What the code already decides

- The engine is synchronous. `run_game_loop` takes an
  `FnMut(&GameState, PlayerId, &LegalActions) -> Action` and calls it once
  per decision; the `Player` trait's `choose_action` blocks until it has an
  answer. A GUI seat is therefore a blocking call that waits on something
  that pumps a screen, whatever that screen is.
- Everything a seat is asked is already a typed value. `LegalActions`
  carries the flat `actions` list, the collapsed `castable_spells` and
  `activatable_abilities` (with their `CastTargetSpec`), the
  `combat_prompt`, the `set_prompt`, and the `resolution_prompt` (seventeen
  `ResolutionChoiceKind`s). `Action`, `CombatPrompt`, `SetPrompt`,
  `ResolutionChoiceKind` and every leaf type already derive `Serialize`;
  `GameView`, `PermanentView`, `LegalActions` do not yet, but only need the
  derive.
- Hidden information is already filtered: `GameView::for_player` is one
  player's view. A GUI that only ever receives a `GameView` cannot leak the
  other hand by construction, which is the property `hidden_information.rs`
  pins for the CLI.
- The card pool is about 270 cards (250 Innistrad, 20 core) plus tokens.
  `data/oracle_cache.json` has the name, mana cost, type line, oracle text
  and P/T of each, which is enough to write an art prompt per card.
- The CLI is 9,900 lines of bespoke screens. A GUI is a fourth surface on
  top of CLI, LLM and random, and `CLAUDE.md`'s rule applies to it: a
  change to what the engine asks has to be walked through the GUI too.
- The nightly playtest crew and `cli_pty.rs` test the CLI by driving a
  real terminal. The GUI needs an equivalent it can be driven through.

## Decision 1: where the pixels are drawn

**Options**

1. A native window in Rust (`macroquad`, `ggez`, `pixels` + `winit`,
   `bevy`). One language, one binary, `--p1 gui` opens a window.
2. A browser page drawn on an HTML canvas, served by the runner. The Rust
   side is a seat plus a small local WebSocket server; the page is the GUI.
3. Everything in the browser: engine compiled to `wasm32`, GUI in the same
   module, no server. Playable as a static page.

**Constraints that decide it**

- The `claude-code` seat is a `claude -p` subprocess and the `claude` and
  `gemini` seats hold API keys; `--save`, `--resume`, `--log`, the watchdog
  and the invariant checks all live in the runner. Option 3 loses all of
  it, or duplicates the runner in JavaScript. It is a nice demo target
  later, not the first build.
- Option 1 needs GL, X or Wayland libraries on the machine that builds and
  runs it. The development container has none of them and no display, so
  nothing built this way can be run, screenshotted or tested by the agents
  that do most of the work on this repo. A browser page can be: Chromium
  and Playwright are pre-installed, and a Playwright test is the GUI's
  `cli_pty.rs`.
- Pixel-perfect integer scaling, a resizable window, hot reload of the
  frontend, text rendering with a bitmap font and a hover inspector are
  all trivial on a canvas and each a small project in a Rust game
  framework.
- Two browser tabs on one server are two seats. Hotseat, spectating and
  "watch two LLM seats play" come for free from option 2 and need
  explicit work in option 1.

**Recommendation: option 2.** The runner grows a `gui` seat kind that
serves one page and one WebSocket on `127.0.0.1`, prints the URL, and
blocks in `choose_action` until the page answers. The page is the whole
GUI. The Rust side is small (a seat, a server, `Serialize` derives) and
the part that changes fastest, the visuals, needs no compile.

The Rust-only alternative that keeps most of option 2's advantages is
`macroquad` built for `wasm32-unknown-unknown` and served the same way;
it stays open as the frontend-language question below, but it loses the
DOM for text and menus and still cannot be tested here without the wasm
target, which is not installed.

## Decision 2: the seam between the engine and the GUI

The LLM seat has its own hand-written prompt and JSON schema, and that
seam is where #398 and #404 happened. The GUI should not get a third
hand-written description of the same prompts.

**Recommendation: the protocol is the engine's own types as JSON.** Each
decision is one message `{ "view": GameView, "legal": LegalActions,
"seat": PlayerId, "seq": n }` and the answer is one `Action`. Nothing is
summarised or renamed on the way out; `serde` derives on `GameView`,
`PermanentView`, `CardView`, `StackItemView`, `OpponentView` and
`LegalActions` are the whole encoding. Consequences:

- A new `ResolutionChoiceKind` is a compile error nowhere but a message
  the page has never seen. So the page has one rule: an unknown prompt
  kind renders as the generic "pick from list" widget with the kind's
  `description`, never as nothing. That is the GUI's version of
  `CLAUDE.md`'s second rule (no silent no-op), and a test asserts it for
  every variant.
- The answer is checked by the engine exactly as a random seat's is;
  the seat accepts the JSON `Action`, and an illegal one is refused the
  way the CLI refuses a bad row, with the prompt re-sent.
- `legal.actions` is a flat list, but the GUI is card-centric (click the
  card, see what it can do). The page indexes the list by `object_id`:
  `PlayLand`, each `CastableSpell`, each `ActivatableAbility` and the
  combat prompt's `eligible` all name the object they hang off, so a
  click on a permanent or a hand card lists exactly its legal verbs.
- Two-slot targeting (`CastTargetSpec::TwoTargets`, where the second
  list depends on the first pick) and `ChooseTargetSet` ("up to N") are
  answered one slot at a time on the board, with the candidates
  highlighted and everything else dimmed, then a Confirm. That is the
  "ask for the set on one screen" rule from `CLAUDE.md`.

## Decision 3: how the prompts map to widgets

Seventeen resolution kinds, the set prompt, two combat prompts, the
mulligan pair and the priority menu. Bespoke screens for each is the CLI's
shape and is why it is 9,900 lines. Five widget shapes cover all of them:

| Widget | Answers |
|---|---|
| Board pick: click one highlighted thing | `ChooseTarget`, `ChosenCard`, `ChooseCardFromHand`, `ChooseFromLookedAt`, `ChooseFromLibrary`, single-target casts |
| Board mark: toggle N..M highlighted things, Confirm | `ChooseTargetSet`, `ChooseObjectSet`, `ChooseExileFromGraveyard`, `SetPrompt` (bottom, discard), `DividePermanentsIntoPiles`, declare attackers |
| Pair: pick a thing, then what it goes on | declare blockers, planeswalker attacks, `TwoTargets` casts |
| List: a small menu of labelled rows | priority menu verbs on a clicked card, `ChooseCardType`, `ChooseDamageEffect`, `ChoosePile`, `ChooseCardName` (with a filter box past eight names), `YesNo`, `PayOrNot`, mulligan keep/mull |
| Order: drag rows, Confirm | `ChooseTriggerOrder`, `ChooseDamageAssignmentOrder` |

Plus one number field for `ChooseXFunding`, defaulting to the auto-tap
plan the CLI already computes.

**Recommendation: build exactly these and nothing per card.** Anything a
card needs that these cannot express is an engine prompt-shape problem
first (`mtg-engine/tests/prompt_shapes.rs` exists for that reason).

## Decision 4: canvas size, card footprint, and what is on screen

**Virtual resolution.** Draw to a fixed virtual canvas and scale it by an
integer to the window. Two credible choices:

- 320x180: chunky, NES-like, scales 4x to 720p and 6x to 1080p. A card
  gets about 24x32 px, so art is 20x14 and names cannot be printed on
  the board.
- 640x360: SNES-like, 2x to 720p, 3x to 1080p. A card gets 40x56 with
  a 36x26 art window, room for P/T and status glyphs, and a hand card can
  be 48x68 at the same art size (one asset, framed differently).

**Recommendation: 640x360.** The board has to hold two players' worth of
permanents, a stack and a hand at once, and at 320x180 it cannot without
scrolling on turn six. Every asset is drawn at 1x and scaled with
`image-rendering: pixelated`; nothing is ever resampled.

**Layout** (top to bottom): opponent strip (life, hand as card backs,
library and graveyard and exile piles with counts, mana pool); opponent
battlefield in three rows (lands, creatures, other permanents); a center
band with the turn and step tracker (untap through cleanup, the current
one lit, the active and priority player marked), the stack as a column
of small cards with lines to their targets, and the pass button; your
battlefield in the same three rows; your hand fanned; your strip. A
right-hand inspector panel shows the hovered card at 3x with its oracle
text, counters, attachments, `protections` and `restrictions`, and its
`granted_abilities`. The log is a drawer along the bottom edge, opened by
`l`, and the graveyards and exile by clicking the pile.

**Overflow.** The playtest guide stresses 30+ permanents a side. Untapped
basic lands of one name collapse into one stack with a count; creature
rows past fourteen shrink to 2/3 width (art still integer-scaled: 1x art
in a narrower frame that clips the sides) and then scroll. A row never
draws outside its band, which is the "everything printed fits" rule.

**Combat.** Attackers slide toward the center band; a blocker is drawn
under the attacker it blocks; damage assignment order is the order they
sit in. Menace and other `min_blockers` are shown as a badge on the
attacker so an under-minimum block is refused on the page before the
engine refuses it.

**Hotseat.** Two humans on one machine is two tabs, one per seat. On one
tab it is a "pass the screen" curtain between decisions, which is the
CLI's behaviour. Not needed for the first build; two tabs is.

## Decision 5: text and symbols

Card names, oracle text, the log and every menu are text. On a canvas at
1x that means a bitmap font.

**Recommendation:** one 5x7 or 6x8 pixel font (an OFL or CC0 one,
`m5x7`, `Silkscreen` or `Press Start 2P`) rendered by the canvas at 1x, so
it scales with everything else and is readable at 2x. Mana symbols are
8x8 icons (W, U, B, R, G, C, generic 0-9, X, tap) drawn inline. Keywords
stay as words. Names are untrusted text (#315); on a canvas they are
only ever glyphs, and the one DOM element, the filter box, is set through
`textContent`.

## Decision 6: the art pipeline

**What to generate.** One art crop per card at one size, 36x26 for the
640x360 layout (or the nearest size the chosen style allows; the styles
endpoint reports per-style limits). The same crop is shown at 3x in the
inspector, so there is no second asset. Beyond cards: ten or so token
kinds, five card backs or one, the table, the card frames per colour and
type, the step tracker, the pile and button chrome, and a set of 8x8
icons. About 300 images.

**Retro Diffusion.** `POST https://api.retrodiffusion.ai/v2/inferences`
with header `X-RD-Token: rdpk-...`, body `prompt`, `prompt_style`,
`width`, `height`, `num_images`, `seed`; the response is a task id polled
at `GET /v2/inferences/tasks/{task_id}` and the result carries
`base64_images` and `balance_cost`. `check_cost: true` estimates without
charging. Styles and rough prices per image: `rd_fast__*` and
`rd_mini__*` about $0.03, `rd_plus__*` about $0.06, `rd_pro__*` about
$0.18. `rd_plus__low_res` or `rd_fast__low_res` are the ones for a
36x26 crop; `rd_plus__ui_element` and `rd_fast__ui` for chrome;
`rd_tile__*` for the table. A full pass at `rd_plus` is roughly $20, at
`rd_fast` roughly $10, and a retry budget on top. There is also a hosted
MCP server at `https://mcp.retrodiffusion.ai/mcp` with the same
capabilities, which is the convenient way to iterate on a handful of
images by hand.

**Prompts from data.** The prompt is built from the oracle cache, not
written per card: name, the type line's types and subtypes, the colour
from the mana cost, and a fixed set-flavour suffix ("gothic horror,
Innistrad, moonlit"), with a per-colour palette hint. That makes the
whole pass a script, `scripts/gen_card_art.py`, that writes
`mtg-gui/assets/art/<slug>.png` and a manifest
`mtg-gui/assets/art/manifest.json` recording card name, prompt, style,
size and seed per image, so any one card can be regenerated alone and
the pass is reproducible. The key comes from `RETRO_DIFFUSION_API_KEY`
and is never written anywhere.

**Placeholder first.** The GUI ships with a procedural placeholder (a
frame in the card's colour with the name's initials and the type icon)
used for any card with no art file. The game is playable and every test
passes with zero generated assets, and CI needs no key. Generated PNGs at
this size are a few hundred bytes each, so committing them is fine.

## Decision 7: repository shape and dependencies

- `mtg-player/src/gui.rs`: the seat. It sits next to `cli.rs`, `llm.rs`
  and `random.rs` because it is the same kind of thing, and the
  `CLAUDE.md` list of surfaces becomes four files in one crate.
- `mtg-gui/`: the page. `index.html`, `src/*.ts`, `assets/`, `tests/`
  (Playwright). TypeScript compiled with `esbuild` (one dev dependency,
  no bundler config); the built `app.js` is embedded into the runner with
  `include_str!` so a release binary is still one file. A plain-JS,
  no-toolchain variant is the alternative if adding `node` to the build
  is unwelcome.
- Rust dependencies: a blocking WebSocket (`tungstenite`) and a tiny
  HTTP server (`tiny_http`) for the static files, keeping the crate
  synchronous like the rest of the workspace. No `tokio`.
- The runner: `--p1 gui[:port]`, `--open` to launch the browser, the
  URL printed on stderr. `--resume` and `--save` work unchanged since the
  seat is just a seat. Hot reload is the page reconnecting: on connect
  the server re-sends the pending decision.

## Decision 8: how it is tested

- `mtg-player/tests/gui_protocol.rs`: every `ResolutionChoiceKind`, the
  set prompt and both combat prompts serialize, and a legal `Action`
  round-trips for each. This is the test that a new prompt kind cannot
  pass without the page being told.
- `mtg-gui/tests/*.spec.ts`: Playwright drives a real runner with
  `--seed` and fixture saves (`--resume` from a save captured at each
  prompt kind, the way `save_files.rs` already builds states) and checks
  the page can answer each, at 1x, 2x and 3x, with screenshots as
  artifacts. Runs in CI with the pre-installed Chromium.
- A `docs/playtest/gui.md` guide once the surface exists, so the nightly
  crew has a fifth subject.

## Phasing

1. Protocol and seat: derives, `gui.rs`, the server, a page that renders
   the board from placeholders and answers the priority menu, casting
   with targets, land drops, combat, mulligan, bottoming and discard.
   Playable end to end against `random` and `claude-code`.
2. The five widgets for every resolution kind, the inspector, the log
   drawer, the graveyard and exile views, the protocol test.
3. The art script and one full generation pass; frames, icons, table.
4. Combat motion, the stack's target lines, two-tab hotseat, the
   Playwright suite in CI, the playtest guide.
5. Later: the draft runner's picks through the same seat, and a
   wasm build for a serverless demo.

## Questions for the owner

1. Browser page served by the runner (recommended) or a native window?
2. Frontend in TypeScript with `esbuild` (recommended) or Rust compiled
   to wasm through `macroquad`?
3. 640x360 virtual resolution (recommended) or 320x180?
4. Art: is there a Retro Diffusion key and a budget of roughly $10 to
   $25 for a full pass, and is `rd_plus` (recommended) or `rd_fast` the
   tier? Or placeholders only until the layout settles?
5. Is two-human hotseat needed in the first build, or is one GUI seat
   against a bot or LLM enough to start?
