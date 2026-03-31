# Audit: Disciple of Griselbrand

## Reference (Scryfall)
- **Name:** Disciple of Griselbrand
- **Cost:** {1}{B}
- **Type:** Creature -- Human Cleric
- **Oracle:** {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
- **P/T:** 1/1

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- Oracle text: CORRECT
- P/T: CORRECT (1/1)
- Activated ability cost {1}: CORRECT
- Sacrifice a creature cost: CORRECT (SacrificeCost::SacrificeCreature)
- requires_tap: CORRECT (false)
- Life gain equals sacrificed creature's toughness: CORRECT (reads from CreatureDied event)

## Issues
None found.
