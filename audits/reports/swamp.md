# Audit: Swamp

## Oracle (Scryfall)
- **Name:** Swamp
- **Cost:** N/A (Land)
- **Type:** Basic Land -- Swamp
- **Oracle:** ({T}: Add {B}.)
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/swamp.rs`
- **Name:** Swamp ✅
- **Cost:** None ✅
- **Type:** Land ✅
- **Supertypes:** Basic ✅
- **Subtypes:** Swamp ✅
- **Mana ability:** {T}: Add {B} ✅
- **requires_tap:** true ✅
- **Zone check:** only when on battlefield and untapped ✅

## Verdict: PASS -- no issues found
