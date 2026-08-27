## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/51/delver-of-secrets-insectile-aberration?utm_source=api
**Type line**: `Creature — Human Wizard` — {U}, 1/1
**Oracle text**:
```
At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
```
**Back face**: Insectile Aberration, `Creature — Human Insect`
```
Flying
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.
Here the duplicate was fused into a larger condition,
`if is_transformed || state.active_player != controller`; only the
`active_player` half was removed, leaving the transform check intact.

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/51/delver-of-secrets-insectile-aberration?utm_source=api
**Type line**: `Creature — Human Wizard` — {U}, 1/1
**Oracle text**:
```
At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
```
**Back face**: Insectile Aberration — `Creature — Human Insect`, 3/2
```
Flying
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You **may** reveal the card even if it's not an instant or sorcery.
  Whether or not you reveal it, the card **stays on top of your library**." The
  prompt is offered regardless of what the top card is, the card is never moved,
  and only an instant or sorcery reveal transforms: PASS
- The player "looks at" the top card before deciding, so the prompt names it —
  the information is theirs by the card's own text: PASS
- Declining leaves the card on top and does not transform: PASS
- An empty library offers the choice and transforms nothing: PASS
- The trigger is on the front face only; Insectile Aberration has no ability:
  PASS
- Flying is on the back face only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The optional reveal and the conditional transform: `cards_transforming_permanents.rs:delver_transforms_when_player_reveals_instant`, `transform_dfc.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/51/delver-of-secrets-insectile-aberration?utm_source=api
**Type line**: `Creature — Human Wizard` — {U}, 1/1
**Oracle text**:
```
At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
```
**Back face**: Insectile Aberration — `Creature — Human Insect`, 3/2
```
Flying
```

**Rulings fetched**:
- [2011-09-22] You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.

**Status**: ISSUE (fixed)

### Code issues

**A reveal prompt was offered with an empty library.**

- Oracle text says: `look at the top card of your library. You may reveal that card.`
- Code did: read the top card as `map_or_else(|| "nothing".into(), ...)` and
  presented the Yes/No regardless, so with no library the controller was asked
  `"Delver of Secrets: reveal nothing from the top of your library? (not an
  instant or sorcery — no transform)"`.

There is no card to look at and nothing to reveal, so there is no choice to
make. CR 608.2 — the ability does as much as it can, which here is nothing. The
handler now returns before prompting. Small, but a decision with nothing behind
it is noise a player (and an LLM player) has to spend a turn answering.

### Rulings checked

- **"You may reveal the card even if it's not an instant or sorcery. Whether or
  not you reveal it, the card stays on top of your library."** Both halves hold.
  The Yes/No is offered whatever the top card is — the description even says
  which case you are in, which the controller legitimately knows because they
  just looked at it — and nothing in either branch moves the card: no
  `move_object`, no `draw_top_card`, no reordering. PASS.

### Tricky interactions checked

- **Hidden information.** This was the one I most expected to find a bug in, and
  it is clean. The top card's name is written to the log at
  `LogLevel::Debug`, and `GameView::for_player` builds `display_log` by filtering
  to `Info` and above. Both players — the CLI at `cli.rs:992` and the LLM at
  `llm.rs:2002` — read `display_log`, never `full_log`. So the opponent does not
  learn the top card of your library. Added a comment saying the Debug level is
  deliberate, since it reads like an accident otherwise. PASS.
- **Which end of the library is the top.** `library_order.first()`, matched
  against `PlayerState::draw_top_card`, which is `library_order.remove(0)`.
  Same end. PASS.
- **A double-faced card on top of the library** is judged by its front face —
  `has_card_type` goes through `face_data`, and a library card is never
  `is_transformed` (CR 712.8a). PASS.
- **The re-read between prompt and answer.** `on_upkeep` looks at the top card
  and `on_yes_no_choice` reads it again. No player receives priority between an
  `awaiting_action` and its answer, so the library cannot change in between.
  PASS.
- **Insectile Aberration is still a Human** — `Creature — Human Insect`, and the
  code has both subtypes. That matters in this set: Moonmist transforms all
  Humans, so it flips a transformed Delver *back*. PASS.
- **Flying belongs to the back face only.** Scryfall lists the card's keywords
  as "Flying, Transform"; flying is printed on Insectile Aberration, and the code
  puts it in `back_face_data` with none on the front. Transform is correctly not
  a keyword here — it is Scryfall's tag for a text pattern, not something
  `has_keyword` should answer to. PASS.
- **The back face is vanilla.** No triggered abilities declared on it, so the
  upkeep look-and-reveal stops once transformed — the collector picks triggers by
  face. The `is_transformed` early return in `on_upkeep` is therefore redundant,
  but harmless, and it documents the intent. PASS.
- **`should_transform` returns false** — Delver is not a Werewolf and never
  flips on a board condition; it flips only through its own reveal. PASS.
- **Killing the Delver in response** is not the Cloistered Youth case: the only
  consequence of this ability is the transform, which needs the permanent, so
  the battlefield guard costs nothing observable. PASS.

### Test coverage

- no prompt with an empty library: `cards_transforming_permanents.rs::delver_offers_no_reveal_when_the_library_is_empty` (new, mutation-checked).
- transforms on revealing an instant or sorcery: `cards_transforming_permanents.rs:47`.
- declining the reveal leaves it a Delver: `:89`.
- revealing a non-instant/sorcery does not transform: `:122`.
- still a Human after transforming: `subtype.rs:656`.
- transformed Delver that dies and is reanimated comes back front-face-up: `dfc_zone_cleanup.rs:81`.

