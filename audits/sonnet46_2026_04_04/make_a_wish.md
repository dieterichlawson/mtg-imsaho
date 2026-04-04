## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Return two cards at random from your graveyard to your hand.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Can't return itself (ruling 1)**: During `on_resolve`, the spell has been popped off `state.stack` (the array) but its game object still resides in `Zone::Stack` (the zone change via `move_spell_after_resolve` happens after). `objects_in_zone(Zone::Graveyard, ...)` only returns objects with `zone == Zone::Graveyard`, so the spell is never in the candidate pool. The code also has an explicit `o.id != object_id` guard (line 36) as redundant safety. PASS.
- **Random selection happens at resolve time (ruling 2)**: The shuffle occurs inside `on_resolve`, which is called at resolution. No pre-selection. PASS.
- **Only 1 card in graveyard returns that 1 card (ruling 3)**: `gy_cards.into_iter().take(2)` on a 1-element vector yields 1 element. PASS.
- **Zero cards in graveyard**: `to_return` would be empty; the `if to_return.is_empty()` block logs appropriately and `move_spell_after_resolve` still runs. PASS.
- **Tokens excluded**: Oracle text says "cards"; the code filters `!o.is_token` before selecting candidates. PASS.
- **"your graveyard" (controller's graveyard)**: Code reads `controller` from the spell object and passes it to `objects_in_zone(Zone::Graveyard, controller)`. That function checks `obj.owner == player` for graveyard zones. Since cards go to their owner's graveyard, `controller == owner` in all normal cases and the two coincide. PASS.
- **Spell cleanup via `move_spell_after_resolve`**: Called on line 58 before `on_resolve` returns, moving the spell to graveyard (or exile if cast with flashback). PASS.
- **Mana cost {3}{G} = CMC 4**: Code uses `ManaSymbol::Generic(3)` + `ManaSymbol::Colored(Color::Green)`. PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Returns exactly 2 cards at random from a graveyard with 3 cards: `innistrad_simple_cards.rs:502` TESTED
- Card type is Sorcery and CMC is 4: `innistrad_simple_cards.rs:493` TESTED
- Can't return itself (ruling 1 — spell is in Zone::Stack, not Zone::Graveyard): NOT TESTED (but logically guaranteed by zone mechanics)
- Cards chosen randomly at resolve time (ruling 2): NOT TESTED (randomness not directly asserted; test only checks count)
- Only 1 card in graveyard returns that 1 card (ruling 3): NOT TESTED
- Zero cards in graveyard: NOT TESTED
- Tokens excluded from candidate pool: NOT TESTED
