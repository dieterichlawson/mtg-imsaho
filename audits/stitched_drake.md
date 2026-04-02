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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.\nFlying
**Mana cost**: {1}{U}{U}
**Type line**: Creature — Zombie Drake
**P/T**: 3/4
**Status**: ISSUE
### Checks
- **Name**: "Stitched Drake" -- CORRECT
- **Mana cost**: Generic(1) + Blue + Blue -- CORRECT ({1}{U}{U})
- **Type**: Creature with Zombie, Drake subtypes -- CORRECT
- **P/T**: 3/4 -- CORRECT
- **Keywords**: Flying -- CORRECT
- **Additional cost**: ExileCreaturesFromGraveyard(1) declared in card_data -- CORRECT
### Code issues
1. **ISSUE — Additional cost executed at resolution instead of cast time**: The `on_resolve` method exiles a creature card from the graveyard. Per oracle, this should happen as an additional cost to *cast* the spell (before it goes on the stack), not when the spell resolves. The card_data correctly declares `AdditionalCost::ExileCreaturesFromGraveyard(1)`, but `on_resolve` redundantly exiles again. If the engine already handles the additional cost at cast time, this causes a double-exile; if not, the timing is wrong.
   - Code on_resolve: searches graveyard for creature, exiles it, then moves self to battlefield
   - Oracle: "As an additional cost to cast this spell, exile a creature card from your graveyard."
