# Audit: Gallows Warden

## Reference (Scryfall)
- **Name:** Gallows Warden
- **Cost:** {4}{W}
- **Type:** Creature -- Spirit
- **Oracle:** Flying. Other Spirit creatures you control get +0/+1.
- **P/T:** 3/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({4}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Spirit)
- Oracle text: CORRECT
- P/T: CORRECT (3/3)
- Keywords: CORRECT (Flying)
- +0/+1 to other Spirit creatures you control: CORRECT (ModifyPT power:0, toughness:1, scope: GlobalOther with You + HasSubtype("Spirit"))

## Issues
None found.
