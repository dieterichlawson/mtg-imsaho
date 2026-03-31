# Audit: Diregraf Ghoul

## Reference (Scryfall)
- **Name:** Diregraf Ghoul
- **Cost:** {B}
- **Type:** Creature -- Zombie
- **Oracle:** Diregraf Ghoul enters the battlefield tapped.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Zombie)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- ETB tapped: CORRECT (sets obj.tapped = true in on_resolve)

## Issues
None found.
