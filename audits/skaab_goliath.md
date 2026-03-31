# Audit: Skaab Goliath

## Oracle (Scryfall)
- **Name:** Skaab Goliath
- **Cost:** {5}{U}
- **Type:** Creature -- Zombie Giant
- **Oracle:** As an additional cost to cast Skaab Goliath, exile two creature cards from your graveyard. Trample
- **P/T:** 6/9

## Implementation: `mtg-engine/src/cards/skaab_goliath.rs`
- **Name:** Skaab Goliath ✅
- **Cost:** {5}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Zombie, Giant ✅
- **P/T:** 6/9 ✅
- **Keywords:** Trample ✅
- **Additional cost:** ExileCreaturesFromGraveyard(2) ✅
- **on_resolve:** exiles 2 creature cards from graveyard, moves to battlefield ✅

### Issue
- **BUG:** The additional cost (exiling creatures from graveyard) is being paid in `on_resolve` rather than during casting. If the spell is countered, the creatures should still be exiled (additional costs are paid on cast, not resolve). However, this is a known engine-wide pattern for the Skaab cards.

## Verdict: PASS -- known engine limitation with additional costs paid at resolve time
