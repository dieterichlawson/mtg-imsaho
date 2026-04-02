# Audit: Lumberknot

## Oracle (Official)
- **Name:** Lumberknot
- **Cost:** {2}{G}{G}
- **Type:** Creature — Treefolk
- **Oracle:** Hexproof. Whenever a creature dies, put a +1/+1 counter on Lumberknot.
- **P/T:** 1/1

## Implementation
- Name: "Lumberknot" -- CORRECT
- Cost: {2}{G}{G} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Treefolk"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Hexproof] -- CORRECT
- Oracle text matches -- CORRECT
- Triggered ability: AnyCreatureDies -- CORRECT
- on_any_creature_dies: adds +1/+1 counter if on battlefield -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lumberknot
- **Cost:** {2}{G}{G}
- **Type:** Creature — Treefolk
- **P/T:** 1/1
- **Oracle Text:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Whenever a creature dies, put a +1/+1 counter on this creature.

### Card Data Checks
- [x] Name: "Lumberknot" — correct
- [x] Cost: {2}{G}{G} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Treefolk — correct
- [x] P/T: 1/1 — correct
- [x] Keywords: Hexproof — correct
- [x] Triggered ability: AnyCreatureDies — correct

### Behavior Checks
- [x] Hexproof keyword present — correct
- [x] `on_any_creature_dies` triggers for any creature dying — correct
- [x] Only adds counter if self is on battlefield — correct
- [x] Adds PlusOnePlusOne counter — correct
- [x] Triggers on any creature (not just own or opponents') — correct per oracle

### Result: PASS
