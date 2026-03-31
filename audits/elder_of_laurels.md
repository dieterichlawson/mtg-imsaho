# Audit: Elder of Laurels

## Reference (Scryfall)
- **Name:** Elder of Laurels
- **Cost:** {2}{G}
- **Type:** Creature -- Human Advisor
- **Oracle:** {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
- **P/T:** 2/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Advisor)
- Oracle text: CORRECT
- P/T: CORRECT (2/3)
- Activated ability cost: CORRECT ({3}{G})
- requires_tap: CORRECT (false)
- Target creature: CORRECT (TargetRequirement::Creature)
- +X/+X where X = creatures you control: CORRECT (counts creatures on battlefield under controller)

## Issues
None found.
