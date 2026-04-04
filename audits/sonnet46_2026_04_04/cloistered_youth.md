## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, you may transform this creature. // At the beginning of your end step, you lose 1 life.
**Type line**: Creature — Human // Creature — Horror
**Status**: ISSUE

### Code issues

- Spurious upkeep trigger fires for Unholy Fiend (transformed state)
  - Oracle text says: `At the beginning of your end step, you lose 1 life.` (Unholy Fiend back face has NO upkeep ability)
  - Code does: `trigger_description` in `mtg-engine/src/triggers.rs` line 314 always checks front face triggers first regardless of `is_transformed`. When the permanent is transformed (`is_transformed=true`) and the step is `Upkeep`, the function finds `TriggerKind::Upkeep` on the front face (Cloistered Youth) and returns `"you may transform Cloistered Youth"` immediately — before the `is_transformed` guard on line 318 is ever reached. `collect_triggers` sees a non-empty description and creates an `UpkeepTrigger` for the transformed permanent, placing it on the stack. The trigger resolves by calling `on_upkeep`, which guards with `if !is_transformed` and returns early — so no game state changes — but the trigger still appears on the stack during Unholy Fiend's controller's every upkeep. This is incorrect: Unholy Fiend has no upkeep ability and no upkeep trigger should ever be generated for it.

### Tricky interactions checked

- "You may" optionality on upkeep transform: `on_upkeep` sets `state.awaiting_action` with a `YesNo` choice, correctly presenting the option to the player rather than auto-selecting — PASS
- Active-player ("your") guard on upkeep trigger: `on_upkeep` checks `state.active_player != controller` and returns early for non-controllers — PASS
- Active-player ("your") guard on end step trigger: `on_end_step` checks `state.active_player != controller` and returns early for non-controllers — PASS
- Zone guard (battlefield only): both `on_upkeep` and `on_end_step` check `o.zone == Zone::Battlefield` before proceeding — PASS
- P/T becomes 3/3 after transform: `dynamic_pt` returns `Some((3, 3))` when `is_transformed=true`; `effective_power`/`effective_toughness` in `state.rs` call `dynamic_pt` for the creature's own card — PASS
- P/T reverts to 1/1 when not transformed: `dynamic_pt` returns `None` when `is_transformed=false`, falling back to `obj.power`/`obj.toughness` (1/1 from card data) — PASS
- `apply_transform` correctly updates name, keywords, subtypes on transform: helper at `helpers.rs:231` flips `is_transformed`, then updates `obj.name`, `obj.keywords`, `obj.subtypes` from the appropriate face — PASS
- Back-face end step trigger fires when transformed: `trigger_description` for `TriggerKind::EndStep` with `is_transformed=true` falls through the front-face check (no EndStep on front face) and correctly finds "lose 1 life" on the back face — PASS
- No end step trigger when not transformed: `trigger_description` for `TriggerKind::EndStep` with `is_transformed=false` finds no EndStep on front face and skips the back-face check (guarded by `is_transformed`) — returns empty; no trigger created — PASS
- Spurious upkeep trigger when transformed as Unholy Fiend: `trigger_description` for `TriggerKind::Upkeep` with `is_transformed=true` hits the front-face check first (line 314), finds the Upkeep trigger, and returns its description before the `is_transformed` guard can prevent it — FAIL (engine bug in `triggers.rs`)
- Life loss event correctly emitted: `on_end_step` pushes `GameEvent::LifeChanged` with correct old/new values — PASS
- Mana cost {1}{W}: `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)])` matches oracle — PASS
- Front face subtypes (Human) / back face subtypes (Horror): correctly declared in `card_data()` and `back_face_data()` respectively, and `apply_transform` updates `obj.subtypes` when transforming — PASS

### Test coverage

- Front-face upkeep choice presented (you may transform): `tier15_cards.rs:1033` — TESTED
- Player can decline transform (chooses no): `tier15_cards.rs:1060` — TESTED
- Back-face end step life loss: `tier15_cards.rs:1085` — TESTED
- Front face card data has exactly one Upkeep trigger: `tier15_cards.rs:1102` — TESTED
- Back face card data has exactly one EndStep trigger: `tier15_cards.rs:1114` — TESTED
- Spurious upkeep trigger for Unholy Fiend (transformed) via full trigger collection path: NOT TESTED (all upkeep tests call `behavior.on_upkeep` directly, bypassing `collect_triggers`/`trigger_description`)
- No upkeep choice presented when already transformed (guard inside on_upkeep): NOT TESTED via collect_triggers path
- P/T is 3/3 after transform: `tier15_cards.rs:1056` — TESTED
- Active-player guard (upkeep and end step): NOT TESTED
