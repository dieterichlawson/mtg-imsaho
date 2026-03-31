# Audit: Stitched Drake

## Oracle (Scryfall)
- **Name:** Stitched Drake
- **Cost:** {1}{U}{U}
- **Type:** Creature -- Zombie Drake
- **Oracle:** Flying. As an additional cost to cast Stitched Drake, exile a creature card from your graveyard.
- **P/T:** 3/4

## Implementation: `mtg-engine/src/cards/stitched_drake.rs`
- **Name:** Stitched Drake ✅
- **Cost:** {1}{U}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Zombie, Drake ✅
- **P/T:** 3/4 ✅
- **Keywords:** Flying ✅
- **Additional cost:** ExileCreaturesFromGraveyard(1) ✅
- **on_resolve:** exiles 1 creature card from graveyard, moves to battlefield ✅

### Note
- Same engine limitation as other Skaab cards: additional cost paid at resolve time.

## Verdict: PASS -- known engine limitation with additional costs
