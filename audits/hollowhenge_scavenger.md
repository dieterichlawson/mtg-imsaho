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

## Audit: Hollowhenge Scavenger
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Elemental
- **Cost:** {3}{G}{G}
- **P/T:** 4/5
- **Oracle:** Morbid -- When this creature enters, if a creature died this turn, you gain 5 life.

### Card Data
- **Name:** Hollowhenge Scavenger -- PASS
- **Cost:** {3}{G}{G} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Elemental -- PASS
- **P/T:** 4/5 -- PASS

### Oracle Text Match
- Code uses old-style "enters the battlefield" vs current oracle "enters". Cosmetic only.
- PASS (minor wording variance, no functional impact)

### Behavior Audit
- **Morbid ETB trigger:** Checks `state.creature_died_this_turn` flag. If true, gains 5 life for controller. Pushes LifeChanged event. -- PASS
- **Life gain amount:** 5 -- PASS

### Result: PASS
