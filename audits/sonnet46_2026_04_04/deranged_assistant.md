## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
**Type line**: Creature — Human Wizard
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Library-empty guard**: If library is empty the ability is not offered. `mana_abilities()` checks `!state.get_player(controller).library_order.is_empty()` before returning the ability. Pass.
- **Summoning sickness**: `mana_abilities()` checks `!obj.summoning_sick`, so a freshly played Deranged Assistant cannot activate. Pass.
- **Mana ability does not use the stack**: Implemented via `mana_abilities` + `on_activate_mana_ability` (not `activated_abilities`), so the ability resolves immediately without priority passes or stack interaction. Correct per MTG rules. Pass.
- **Irreversibility ruling**: The ruling states the mill cannot be reversed even if the spell being cast fails. Because mana abilities never go on the stack, the mill is applied atomically inside `submit_action` and can never be "taken back" by game action. Pass.
- **Mill operates on controller's library**: `on_activate_mana_ability` reads `obj.controller` and passes it to `mill_cards(state, controller, 1)`. The card says "your" library, which is the controller's. Pass.
- **Mill moves top card to graveyard**: `mill_cards` does `player_state.library_order.remove(0)` (index 0 = top) then `state.move_object(card_id, Zone::Graveyard)`. Correct. Pass.
- **{C} produces colorless mana**: `produced: vec![(ManaType::Colorless, 1)]`. Matches oracle `Add {C}`. Pass.
- **Tap cost handled**: `requires_tap: true` causes the engine to set `tapped = true` and fire `GameEvent::Tapped` in `ActivateManaAbility` handler. Pass.
- **Ability index consistency**: `ManaAbilityDef { ability_index: 0, ... }` is pushed as `Action::ActivateManaAbility { ability_index: 0 }`, and `submit_action` retrieves it via `abilities.get(0)`. Consistent. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Mana ability offered when library non-empty: `mtg-engine/tests/innistrad_simple_cards.rs:312` (deranged_assistant_taps_for_colorless)
- Mana ability produces 1 Colorless: `mtg-engine/tests/innistrad_simple_cards.rs:329` (asserts `mana_pool.get(ManaType::Colorless) == 1`)
- Card is actually milled when ability activates: NOT TESTED (test does not assert a card moved to graveyard)
- Mana ability not offered when library is empty: NOT TESTED
- Summoning sickness blocks the ability: NOT TESTED
- Card data (P/T, cost, subtypes): `mtg-engine/tests/innistrad_simple_cards.rs:300` (deranged_assistant_card_data)
- Irreversibility of mill: NOT TESTED (ruled as inherent to mana ability mechanics; no dedicated test needed)
