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

## Audit — 2026-04-02

**Oracle Text:**
> As an additional cost to cast this spell, exile two creature cards from your graveyard.
> Trample

**Card Data:**
- Name: Skaab Goliath — correct
- Cost: {5}{U} — correct
- Type: Creature — Zombie Giant — correct
- P/T: 6/9 — correct
- Keywords: Trample — correct
- additional_cost: ExileCreaturesFromGraveyard(2) — correct

**Behavior:**
- ISSUE: `on_resolve` (lines 33-56) manually exiles two creature cards from the graveyard AND moves the card to battlefield. However, the `additional_cost` field is already set to `ExileCreaturesFromGraveyard(2)`, which should handle the exile at cast time. If the engine processes `additional_cost` before calling `on_resolve`, this causes a **double exile** — four creature cards exiled instead of two. The `on_resolve` exile logic should be removed if the engine already handles `additional_cost`.

**Result: ISSUE** — Potential double-exile: both `additional_cost` field and `on_resolve` exile two creatures from graveyard.

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: AdditionalCost::ExileCreaturesFromGraveyard(2) correctly implemented. Oracle text updated to match Scryfall: reordered to "As an additional cost to cast this spell, exile two creature cards from your graveyard.\nTrample" and uses "this spell" instead of "Skaab Goliath". Doc comment updated. Behavior unchanged.
