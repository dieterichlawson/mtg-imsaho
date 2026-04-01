# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

## Open

- [ ] **All equipment cards** — Most equipment uses `TargetRequirement::Creature` for equip instead of `CreatureWithFilter(YouControl)`. Equip by definition targets "creature you control." Fixed for Blazing Torch; other equipment (Cobbled Wings, Mask of Avacyn, Butcher's Cleaver, etc.) likely have the same issue.
