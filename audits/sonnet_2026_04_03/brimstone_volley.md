## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Keywords field incomplete at `mtg-engine/src/cards/isd/brimstone_volley.rs:25`
  - Oracle text says: `Keywords: Morbid`
  - Code does: `keywords: vec![]` (empty vector instead of listing Morbid)

- LLM card knowledge incomplete at `mtg-player/src/llm.rs:83`
  - Oracle text says: `Brimstone Volley deals 3 damage to any target. Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.`
  - Code does: `"Deal 3 damage to any target."` (missing Morbid ability entirely)

### Tricky interactions checked
- Morbid timing throughout the turn: PASS — `creature_died_this_turn` flag persists for entire turn once set, correctly enabling morbid from any previous creature death
- Token creature deaths enabling morbid: PASS — `sba.rs:96,144` and `destruction.rs:100` set the flag for all creature deaths including tokens
- Morbid flag reset at turn boundaries: PASS — `engine.rs:2888` resets `creature_died_this_turn = false` at start of each turn 
- Damage amount calculation: PASS — correctly returns 5 when morbid active, 3 when inactive
- "Any target" flexibility: PASS — `TargetRequirement::AnyTarget` allows targeting players, creatures, and planeswalkers
- Spell cleanup after resolution: PASS — `resolve_damage` calls `move_spell_after_resolve` properly
- NonCombatDamageDealt event type: PASS — uses correct event type for spell damage (not combat damage)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid enabled (5 damage): `mtg-engine/tests/card_mechanics.rs:60` / TESTED
- Morbid disabled (3 damage): `mtg-engine/tests/card_mechanics.rs:75` / TESTED  
- Basic damage to player: `mtg-engine/tests/tier2_spells.rs:48` / TESTED
- Targeting creatures/planeswalkers: NOT TESTED
- Morbid timing edge cases: NOT TESTED
- Token deaths enabling morbid: NOT TESTED
- Multiple creatures dying simultaneously: NOT TESTED