# Audit: Elder Cathar

## Reference (Scryfall)
- **Name:** Elder Cathar
- **Cost:** {2}{W}
- **Type:** Creature -- Human Soldier
- **Oracle:** When Elder Cathar dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Soldier)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Dies trigger: CORRECT (TriggerKind::SelfDies)
- +1/+1 counter on target creature: CORRECT
- Human bonus (2 counters instead of 1): CORRECT
- Targets creature you control: CORRECT (filters by controller)

## Issues
None found.
