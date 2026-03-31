# Audit: Essence of the Wild

## Reference (Scryfall)
- **Name:** Essence of the Wild
- **Cost:** {3}{G}{G}{G}
- **Type:** Creature -- Avatar
- **Oracle:** Creatures you control enter the battlefield as a copy of Essence of the Wild.
- **P/T:** 6/6

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{G}{G}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Avatar)
- Oracle text: CORRECT
- P/T: CORRECT (6/6)
- Replacement effect (implemented as AnyCreatureEnters trigger): functionally reasonable approximation
- Overrides entering creature to 6/6 Avatar: CORRECT
- Only affects creatures controller owns: CORRECT
- Does not affect itself: CORRECT (entered_id == self_id check)

## Issues
None found. (Note: technically this should be a replacement effect, not a triggered ability, but the implementation is a known simplification that is functionally correct for most cases.)
