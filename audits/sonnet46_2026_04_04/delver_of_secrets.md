## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard (front) / Creature — Human Insect (back, Insectile Aberration)
**Status**: ISSUE

### Code issues

- **"You may reveal" choice suppressed when top card is not an instant or sorcery** (`mtg-engine/src/cards/isd/delver_of_secrets.rs` lines 104–118)
  - Oracle text says: `"You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature."`
  - Ruling says: `"You may reveal the card even if it's not an instant or sorcery."`
  - Code does: `if top_is_instant_or_sorcery { state.awaiting_action = Some(AwaitingAction::ResolutionChoice { ... }); } // If not an instant or sorcery, nothing happens`
  - The choice to reveal is gated on the top card being an instant or sorcery. The oracle text presents "you may reveal" as an unconditional option; only the *transform* consequence is conditional. When the top card is a non-instant/non-sorcery, the player is never offered the reveal action at all.
  - The test `delver_does_not_transform_when_top_card_is_creature` (line 1027) actively enshrines this wrong behavior: `assert!(state.awaiting_action.is_none(), "No choice should be presented for non-instant/sorcery")`

- **Transformed Insectile Aberration gets a spurious upkeep trigger on the stack** (engine bug in `mtg-engine/src/triggers.rs` lines 311–327, affecting `delver_of_secrets.rs`)
  - Oracle text for Insectile Aberration says: `"Flying"` — no upkeep trigger.
  - Code does: `trigger_description` in `triggers.rs` always checks front-face triggers first regardless of `is_transformed`: `// Check front face triggers. if let Some(t) = behavior.card_data().triggered_abilities.iter().find(|t| &t.kind == kind) { return t.description.clone(); }`. When Delver is transformed (`is_transformed = true`) and `StepStarted { step: Upkeep }` fires, `trigger_description` finds the front-face `TriggerKind::Upkeep` entry with description `"look at top card, may reveal to transform"` and returns it as non-empty. An `UpkeepTrigger` is then pushed onto the stack for Insectile Aberration. The trigger resolves as a no-op because `on_upkeep` checks `is_transformed` and returns early, but the trigger still visibly appears on the stack labeled "Delver of Secrets's upkeep trigger (look at top card, may reveal to transform)", which is wrong behavior — Insectile Aberration has no upkeep trigger at all.

### Tricky interactions checked

- **"You may reveal" is unconditional**: FAIL — the choice is only offered when the top card is an instant or sorcery; the reveal option is suppressed entirely for other card types (see Issue 1 above).
- **Card stays on top of library after reveal**: PASS — neither branch of `on_yes_no_choice` moves the top card; the library_order is not modified.
- **No transform on decline**: PASS — `on_yes_no_choice` returns early when `yes == false` without setting `is_transformed`.
- **Trigger only fires on controller's upkeep ("your upkeep")**: PASS — `on_upkeep` checks `state.active_player != controller` and returns early when it is not the controller's upkeep.
- **Upkeep trigger not queued when already transformed**: FAIL — `trigger_description` returns a non-empty string for transformed Delver (because it checks front-face triggers first), causing a spurious UpkeepTrigger to appear on the stack, even though Insectile Aberration has no upkeep trigger (see Issue 2 above).
- **Back face data (Insectile Aberration)**: PASS — name, subtypes (Human/Insect), P/T (3/2), keyword (Flying), and oracle_text ("Flying") all match Scryfall.
- **Front face data (Delver of Secrets)**: PASS — name, mana cost ({U}), card types (Creature), subtypes (Human/Wizard), P/T (1/1) all match Scryfall. Front face keywords vec is empty (correct; Flying is only on the back face, and Transform is an ability word not a keyword ability in the engine).
- **dynamic_pt for transformed state**: PASS — returns `Some((3, 2))` when `is_transformed` is true, `None` otherwise.
- **YesNo choice dispatch to on_yes_no_choice**: PASS — engine.rs lines 1995–2001 correctly dispatch `ResolvedChoice::PayDecision(yes)` to `behavior.on_yes_no_choice` for `ResolutionChoiceKind::YesNo`.
- **Source card identity in on_yes_no_choice**: PASS — `source_card` is set to `self_id` in the `YesNo` struct (line 113), and `on_yes_no_choice` receives `self_id` correctly.

### Test coverage

- "You may reveal" choice offered unconditionally (regardless of top card type): NOT TESTED — test `delver_does_not_transform_when_top_card_is_creature` actively asserts the opposite (wrong) behavior at line 1027.
- Player reveals instant/sorcery and Delver transforms: `tier15_cards.rs:938` (tested)
- Player declines to reveal: `tier15_cards.rs:977` (tested)
- No transform when top card is not instant/sorcery: `tier15_cards.rs:1010` (tested, but asserts wrong behavior re: choice suppression)
- Card stays on top of library after reveal: `tier15_cards.rs:973` (tested)
- Card stays on top of library when player declines: `tier15_cards.rs:1006` (tested)
- Transformed Insectile Aberration does not generate upkeep trigger: NOT TESTED
- Trigger only fires on controller's upkeep (not opponent's): NOT TESTED
- Back face Flying keyword active when transformed: NOT TESTED
