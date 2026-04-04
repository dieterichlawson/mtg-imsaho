## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Enchanted player's upkeep timing**: PASS - on_upkeep() correctly checks `state.active_player != cursed_player` and returns early, ensuring trigger only fires when enchanted player is active player (their turn)
- **Insufficient library cards**: PASS - mill_cards() breaks loop when library is empty, correctly milling only available cards per ruling: "If the enchanted player has only one card in their library, they put that card into their graveyard"
- **Target player selection**: PASS - resolve_curse() helper correctly sets attached_to_player from Target::Player, allowing curse to target any player including the caster
- **Curse removed mid-resolution**: PASS - trigger system in triggers.rs re-verifies zone == Battlefield before calling on_upkeep, preventing execution if curse is destroyed
- **Multiple curse copies**: PASS - engine collects each curse's trigger separately, so multiple curses on same player would each mill 2 cards
- **Mill zero cards edge case**: PASS - mill_cards() handles empty library gracefully by breaking early from the loop

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic mill 2 cards functionality**: `mtg-engine/tests/tier7_cards.rs:271` - curse_of_bloody_tome_mills_on_upkeep() tests curse attached to P1 during P1's upkeep mills exactly 2 cards
- **Insufficient library cards ruling**: NOT TESTED - no test verifies behavior when library has fewer than 2 cards
- **Wrong player's upkeep (should not trigger)**: NOT TESTED - no test verifies curse doesn't trigger on other players' upkeeps
- **Target player flexibility (can target self)**: NOT TESTED - no test verifies curse can target the caster
- **Multiple curse copies**: NOT TESTED - no test with multiple curses on same player
- **Mill function edge cases**: `mtg-engine/tests/flashback.rs:166` - mill_cards_moves_to_graveyard() tests mill function itself