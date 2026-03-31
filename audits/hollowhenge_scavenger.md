# Audit: Hollowhenge Scavenger

## Oracle (Official)
- **Name:** Hollowhenge Scavenger
- **Cost:** {3}{G}{G}
- **Type:** Creature — Elemental
- **Oracle:** Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life.
- **P/T:** 4/5

## Implementation
- Name: "Hollowhenge Scavenger" -- CORRECT
- Cost: {3}{G}{G} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Elemental"] -- CORRECT
- P/T: 4/5 -- CORRECT
- Oracle text matches -- CORRECT
- Morbid check uses `state.creature_died_this_turn` -- CORRECT
- Gains 5 life on ETB if morbid -- CORRECT
- Emits LifeChanged event -- CORRECT

## Issues
None.

## Verdict: PASS
