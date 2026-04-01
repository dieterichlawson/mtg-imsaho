## Audit — 2026-04-01

**Scryfall Oracle text (front)**: {T}: Daybreak Ranger deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform Daybreak Ranger.
**Scryfall Oracle text (back)**: {R}, {T}: Nightfall Predator fights target creature.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Nightfall Predator.
**Scryfall type line**: Creature — Human Archer Ranger Werewolf // Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Status**: ISSUE

1. **Nightfall Predator target restriction too narrow** (`mtg-engine/src/cards/daybreak_ranger.rs`, line 128): The `is_valid_target` method for Nightfall Predator restricts targets to `obj.controller != caster` (only opponent's creatures). However, Oracle says "{R}, {T}: Nightfall Predator fights target creature" with no restriction — you can fight any creature, including your own. The code should allow targeting any creature on the battlefield.
