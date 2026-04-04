## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior  
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- ETB trigger timing: PASS — Uses `TriggerKind::EntersBattlefield` and `on_enter_battlefield`, correctly triggered when creature enters battlefield
- Mill fewer than 4 cards edge case: PASS — `mill_cards` function correctly breaks loop when library is empty, handling the ruling "If you have fewer than four cards in your library when Armored Skaab enters, you'll put all of them into your graveyard"
- Mandatory mill effect: PASS — No "may" in oracle text, implementation calls `mill_cards` directly without player choice
- Controller mills own library: PASS — Gets controller properly and passes to `mill_cards(state, controller, 4)`
- Trigger source persistence: PASS — Trigger resolution checks if object is still on battlefield before executing, per MTG rules

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic mill functionality: `flashback.rs:166` (mill_cards_moves_to_graveyard test)
- Mill fewer than 4 cards ruling: NOT TESTED (but mill_cards function logic handles it correctly)
- ETB trigger mechanism: `innistrad_simple_cards.rs:43` (general ETB trigger processing pattern)
- Armored Skaab specific behavior: NOT TESTED