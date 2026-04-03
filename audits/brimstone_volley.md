## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Morbid damage (5 vs 3): PASS — `on_resolve` checks `state.creature_died_this_turn`; returns 5 if true, 3 if false
- Targeting "any target" (creatures and players): PASS — `TargetRequirement::AnyTarget` allows both creatures on battlefield and players; `resolve_damage` handles both via `Target::Object` and `Target::Player` branches
- Morbid flag reset at turn start: PASS — `engine.rs:2888` resets `creature_died_this_turn = false` at start of each turn
- Creature death sets morbid flag: PASS — `destruction.rs:100` and `sba.rs:96,144` set `creature_died_this_turn = true` when creatures die

### Test coverage
- 3 damage to player (no morbid): `tier2_spells.rs:48` and `card_mechanics.rs:75`
- 5 damage to player (morbid active): `card_mechanics.rs:60`
- Damage to creature: NOT TESTED (but `resolve_damage` helper is shared with other cards like Lightning Bolt which test this path)
- Morbid flag reset at turn boundary: `card_mechanics.rs:41`
- Morbid flag set on creature death via SBA: `card_mechanics.rs:26`
