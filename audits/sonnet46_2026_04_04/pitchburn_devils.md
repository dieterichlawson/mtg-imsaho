## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature dies, it deals 3 damage to any target.
**Type line**: Creature — Devil
**Status**: ISSUE

### Code issues

- `any_targets` helper omits planeswalkers; Pitchburn Devils' trigger cannot target them
  - Oracle text says: `it deals 3 damage to any target`
  - Code does: `let targets = crate::cards::helpers::any_targets(state);` which calls `creature_targets` (filtered by `o.power.is_some()`) plus all players — but planeswalkers have `power: None` and are stored as `CardType::Planeswalker` objects, so they are silently excluded. File: `mtg-engine/src/cards/helpers.rs:182-188` and `mtg-engine/src/cards/isd/pitchburn_devils.rs:39`. In MTG, "any target" means any creature, planeswalker, or player/opponent. Since planeswalkers are implemented (Liliana of the Veil, Garruk Relentless — both with `power: None`), a Pitchburn Devils dying while the opponent controls a planeswalker will not offer that planeswalker as a valid target.

### Tricky interactions checked

- **Trigger fires despite source leaving battlefield**: The `SelfDies` trigger dispatched in `triggers.rs:401-415` captures `dead_card_id` and `dead_controller` at the time of death, then `resolve_next_trigger` calls `on_dies` with no battlefield check (correct — the creature is already dead). Pass.
- **`controller_of` after death**: `on_dies` calls `controller_of(state, object_id)` to determine who controls the triggered ability. The object remains in `state.objects` with `zone == Zone::Graveyard` after `move_object`; `state.get_object` finds it and returns the controller correctly. Pass.
- **Dead creature excluded from its own target list**: `any_targets` calls `creature_targets` which filters `o.zone == Zone::Battlefield`. Pitchburn Devils is in the graveyard by the time `on_dies` fires, so it cannot be targeted by its own ability. Pass.
- **Mandatory targeting**: Oracle text has no "may"; the code passes `optional: false` to `present_target_choice`, making it mandatory. Pass.
- **Damage amount**: Oracle says 3; `PendingEffect::DealDamage { amount: 3, … }` matches. Pass.
- **Damage event type**: Uses `GameEvent::NonCombatDamageDealt` (not `CombatDamageDealt`), correct for a triggered ability. Pass.
- **`any target` includes planeswalkers**: Oracle text says "any target" which includes creatures, planeswalkers, and players. `any_targets` helper only includes `o.power.is_some()` objects (creatures) and players. Planeswalkers (Liliana, Garruk Relentless) have `power: None` and are missed. FAIL — see Code Issues.
- **APNAP ordering for trigger**: When Pitchburn Devils dies, `triggers.rs` creates a `SelfDies` trigger assigned to `dead_controller`. APNAP ordering is applied correctly. Pass.
- **Simultaneous deaths**: If Pitchburn Devils dies in a board wipe alongside other creatures, one `CreatureDied` event is emitted per dead creature. Each emits exactly one `SelfDies` trigger for itself. Pitchburn Devils' trigger fires once. Pass.
- **Ruling — life total reaches 0 simultaneously with Pitchburn Devils taking lethal damage**: The trigger goes on the stack after SBAs; if the controller lost from simultaneous damage the game ends before the trigger resolves. The engine processes SBAs (check_state_based_actions) before `process_triggers`; no special handling is needed beyond what SBAs already do. Pass — no code action required.

### Test coverage

- Basic die-trigger (deals 3 damage to opponent): `mtg-engine/tests/tier3_cards.rs:250` (`pitchburn_devils_deals_3_on_death`) — TESTED
- Target choice with multiple targets including a creature: `mtg-engine/tests/card_mechanics.rs:687` (`pitchburn_devils_choice_with_targets`) — TESTED
- Planeswalker as a valid "any target" when Devils dies: NOT TESTED
- Mandatory targeting (cannot decline): NOT TESTED explicitly (implicit in the two tests above)
- Ruling — simultaneous lethal damage to controller and Devils: NOT TESTED
