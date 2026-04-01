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

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- ISSUE (BUG): on_resolve at line 71 does `obj.damage_marked += 13` but does NOT call `obj.damaged_by.push(object_id)`. The resolve_damage helper (helpers.rs:55) correctly pushes to damaged_by, but Into the Maw of Hell implements damage manually without tracking the damage source. This breaks cards like Abattoir Ghoul that check damaged_by for death triggers. Fix: add `obj.damaged_by.push(object_id);` after line 71 in into_the_maw_of_hell.rs.
- All other data (cost, types, two targets, destroy + damage, NonCombatDamageDealt event) is correct.
