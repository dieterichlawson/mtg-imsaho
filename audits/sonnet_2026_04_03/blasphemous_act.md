## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Cost reduction mechanics: PASS - Correctly counts all creatures using `o.zone == Zone::Battlefield && o.power.is_some()`, applies reduction with minimum cost of {R}, and locks in cost before payment
- Creature identification for both cost and damage: PASS - Uses consistent `power.is_some()` check which properly identifies creatures including tokens
- Damage distribution to each creature: PASS - Deals 13 damage to each creature without targeting, using NonCombatDamageDealt events
- Spell cleanup after resolution: PASS - Calls `move_spell_after_resolve(object_id)` properly
- Minimum cost enforcement: PASS - Uses `creature_count.min(8)` to ensure cost cannot go below {R}

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic damage dealing (13 to each creature): `tier12_cards.rs:409-430` 
- Cost reduction with various creature counts: `tier12_cards.rs:434-456`
- Minimum cost of {R} with many creatures: `tier12_cards.rs:450-456`
- Castability with cost reduction: `tier12_cards.rs:459-481`
- Cost calculation timing and locking: NOT TESTED (but implementation follows engine pattern)
