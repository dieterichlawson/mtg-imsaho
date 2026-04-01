## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Daybreak Ranger) {T}: Daybreak Ranger deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform Daybreak Ranger.
(Back — Nightfall Predator) {R}, {T}: Nightfall Predator fights target creature.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Nightfall Predator.
**Scryfall type line**: (Front) Creature — Human Archer Werewolf // (Back) Creature — Werewolf
**Status**: ISSUE

### Findings

1. **Front face subtypes include "Ranger" (ISSUE)**: Implementation has subtypes `["Human", "Archer", "Ranger", "Werewolf"]` (line 34). Oracle type line is "Creature — Human Archer Werewolf" — there is no "Ranger" subtype. "Ranger" is part of the card name, not a creature type.

2. **Nightfall Predator fight target restriction (ISSUE)**: The `is_valid_target` for transformed mode (line 128) restricts to `obj.controller != caster` (creatures you don't control). Oracle text for Nightfall Predator says "fights target creature" with no controller restriction — it can target any creature.

3. **Card data mostly correct**: Name, cost ({2}{G}), type (Creature), P/T (2/2 front, 4/4 back) match.

4. **Werewolf transform logic correct**: Checks if no spells were cast last turn (transform to back) or if any player cast 2+ spells (transform to front).

5. **Damage event correct**: Uses `NonCombatDamageDealt` for the front face ability.

6. **Tests**: No dedicated tests found.
