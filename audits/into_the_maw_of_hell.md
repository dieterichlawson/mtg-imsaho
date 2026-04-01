## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {4}{R}{R}: correct
- Card type Sorcery: correct
- Two targets: one land, one creature (TwoTargets requirement): correct
- On resolve: destroys the land target, then deals 13 damage to the creature target: correct
- Damage emits NonCombatDamageDealt event: correct
- Tests exist in innistrad_simple_cards.rs covering card data
