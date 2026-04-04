## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- `AnyTarget` in engine does not include planeswalkers as valid targets — `mtg-engine/src/engine.rs` lines 836–864, 1074–1089, 1343–1358
  - Oracle text says: `"deals 3 damage to any target"` — per MTG rules "any target" means any creature, player, or planeswalker.
  - Code does: All three `AnyTarget` branches in `generate_cast_actions_with_targets` and the two helper functions filter objects with `o.power.is_some()` (creatures only) and add players, but never include planeswalkers. Planeswalkers (e.g., Liliana of the Veil, Garruk Relentless) have `power: None` and `card_types: vec![CardType::Planeswalker]`, so they are excluded. The engine has a separate `PlayerOrPlaneswalker` requirement that correctly handles planeswalkers but `AnyTarget` does not include the same planeswalker loop.

### Tricky interactions checked

- Morbid condition evaluated at resolution (not cast time): PASS — `on_resolve` reads `state.creature_died_this_turn` at resolution time, which is correct.
- `creature_died_this_turn` set by all creature death paths: PASS — set in `destruction.rs:100` (destroy pipeline), `sba.rs:96` (zero toughness), and `sba.rs:144` (lethal damage fallback).
- `creature_died_this_turn` reset at turn start: PASS — `engine.rs:2888` resets the flag when the turn number advances.
- "any target" includes players: PASS — engine's `AnyTarget` case includes all non-lost players.
- "any target" includes creatures: PASS — engine's `AnyTarget` case includes all battlefield objects with `power.is_some()`.
- "any target" includes planeswalkers: FAIL — engine's `AnyTarget` case omits planeswalkers; the engine has implemented Liliana of the Veil and Garruk Relentless as planeswalkers with `power: None`, making them untargetable by Brimstone Volley.
- Damage uses `NonCombatDamageDealt` event: PASS — `helpers::resolve_damage` emits `GameEvent::NonCombatDamageDealt`.
- Spell moved correctly after resolve: PASS — `helpers::resolve_damage` calls `state.move_spell_after_resolve`.
- Mana cost `{2}{R}`: PASS — `Generic(2), Colored(Color::Red)`.
- Card type Instant: PASS — `vec![CardType::Instant]`.
- "Morbid" absence from `keywords` vec: PASS — "Morbid" is an ability word, not a keyword ability; its absence from the `keywords` vec is correct.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Morbid deals 5 damage when `creature_died_this_turn` is true: `card_mechanics.rs:60` TESTED
- No morbid deals 3 damage: `card_mechanics.rs:75` TESTED
- Basic 3 damage to player: `tier2_spells.rs:48` TESTED
- `creature_died_this_turn` set on creature death via SBA: `card_mechanics.rs:28` TESTED
- `creature_died_this_turn` resets on new turn: `card_mechanics.rs:43` TESTED
- Targeting a planeswalker with Brimstone Volley: NOT TESTED
- Damage dealt to creature (not just player): NOT TESTED for Brimstone Volley specifically
