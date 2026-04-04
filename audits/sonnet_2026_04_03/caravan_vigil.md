## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "may" optionality in Morbid: PASS — Code presents YesNo choice when `creature_died_this_turn` is true, allowing player to decline
- "that card" reference: PASS — Code stores found land ID in card_state and correctly retrieves it in on_yes_no_choice
- "if a creature died this turn" condition: PASS — Code checks `state.creature_died_this_turn` which is correctly maintained by engine
- Basic land search requirements: PASS — Code searches for cards with both `CardType::Land` and `Supertype::Basic`
- Library shuffling after search: PASS — Code shuffles library in both normal case and after morbid choice
- "reveal it" mechanic: PASS — Consistently handled as narrative in this codebase (same as Traveler's Amulet), with mechanical reveals logged when relevant (as in Mulch)
- Spell resolution cleanup: PASS — Code correctly uses `move_spell_after_resolve` in all execution paths
- Morbid choice presentation: PASS — Choice description clearly indicates battlefield vs hand options
- Keywords field: PASS — Morbid is an ability word, not a keyword, so empty keywords vec is correct

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic land search (no morbid): `tier11_cards.rs:170` 
- Morbid battlefield choice (player chooses yes): `tier11_cards.rs:188`
- Morbid hand choice (player chooses no): NOT TESTED
- No basic land found in library: NOT TESTED
- Library shuffling: NOT TESTED (implicit in existing tests)
- "You can choose to put the basic land card into your hand even if a creature died" ruling: PARTIALLY TESTED (only battlefield choice tested)