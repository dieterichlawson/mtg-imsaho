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
