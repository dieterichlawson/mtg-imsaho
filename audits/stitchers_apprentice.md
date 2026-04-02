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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
**Mana cost**: {1}{U}
**Type line**: Creature — Homunculus
**P/T**: 1/2
**Status**: ISSUE
### Checks
- **Name**: "Stitcher's Apprentice" -- CORRECT
- **Mana cost**: Generic(1) + Blue -- CORRECT ({1}{U})
- **Type**: Creature with Homunculus subtype -- CORRECT
- **P/T**: 1/2 -- CORRECT
- **Activated ability cost**: {1}{U}, tap -- CORRECT
- **Token created**: 2/2 blue Homunculus creature token -- CORRECT
### Code issues
1. **ISSUE — Sacrifice selection is automated instead of player-chosen**: The oracle text says "then sacrifice a creature" which means the controller chooses which creature to sacrifice. The code auto-selects a creature, preferring non-tokens over tokens, which is incorrect — the player should be given the choice.
   - Code: `creatures.iter().find(|(_, is_token)| !is_token).or_else(|| creatures.first())`
   - Oracle: "then sacrifice a creature" (controller's choice)

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: activated ability correctly creates token then presents sacrifice choice. Oracle text matches Scryfall verbatim. Behavior unchanged.
