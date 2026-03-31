# Audit: Stitcher's Apprentice

## Oracle (Scryfall)
- **Name:** Stitcher's Apprentice
- **Cost:** {1}{U}
- **Type:** Creature -- Homunculus
- **Oracle:** {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/stitchers_apprentice.rs`
- **Name:** Stitcher's Apprentice ✅
- **Cost:** {1}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Homunculus ✅
- **P/T:** 1/2 ✅
- **Activated ability:** {1}{U}, {T} ✅
- **Token:** 2/2 blue Homunculus, subtypes ["Homunculus"] ✅
- **Sacrifice:** uses `crate::destruction::sacrifice` ✅

### Issue
- **SIMPLIFICATION:** The creature to sacrifice is auto-selected (prefers non-tokens, then tokens) rather than letting the player choose. Oracle says "sacrifice a creature" which should allow player choice.

## Verdict: PASS -- minor simplification in sacrifice target selection
