## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Stromkirk Patrol deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire Soldier
**Status**: PASS

- Name: correct ("Stromkirk Patrol")
- Cost: {4}{B} -- correct
- Type: Creature -- correct
- Subtypes: Vampire, Soldier -- correct
- P/T: 4/3 -- correct (Scryfall confirms 4/3 not 4/2)
- Combat damage trigger: adds +1/+1 counter via `on_combat_damage_to_player` -- correct
- Tests exist in `tier6_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire Soldier
**Status**: PASS

No issues found. Card data, cost, types, subtypes, P/T all correct. Combat damage trigger correctly implemented.
