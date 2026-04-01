## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
**Scryfall type line**: Creature — Demon
**Mana cost**: {3}{B}{B}{B}
**P/T**: 6/6
**Status**: PASS

Implementation correctly models:
- Name, mana cost {3}{B}{B}{B}, type Creature, subtype Demon, P/T 6/6
- Flying keyword
- Morbid triggered ability at end step checking `creature_died_this_turn`
- Targets non-Demon creatures only (filters out Demons by subtype)
- Presents target choice to controller
- Tests: Not found for this specific card, but trigger/morbid infrastructure is tested elsewhere.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
**Scryfall type line**: Creature — Demon
**Status**: PASS

No issues found. Morbid condition checked via creature_died_this_turn. Correctly filters non-Demon targets. Mandatory ability with EndStep trigger.
