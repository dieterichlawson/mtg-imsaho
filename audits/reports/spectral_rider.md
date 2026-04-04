# Audit: Spectral Rider

## Oracle (Scryfall)
- **Name:** Spectral Rider
- **Cost:** {W}{W}
- **Type:** Creature -- Spirit Knight
- **Oracle:** Intimidate
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/spectral_rider.rs`
- **Name:** Spectral Rider ✅
- **Cost:** {W}{W} ✅
- **Type:** Creature ✅
- **Subtypes:** Spirit, Knight ✅
- **P/T:** 2/2 ✅
- **Keywords:** Intimidate ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Mana cost**: {W}{W}
**Type line**: Creature — Spirit Knight
**P/T**: 2/2
**Status**: PASS
### Checks
- **Name**: "Spectral Rider" -- CORRECT
- **Mana cost**: White + White -- CORRECT ({W}{W})
- **Type**: Creature with subtypes Spirit, Knight -- CORRECT
- **P/T**: 2/2 -- CORRECT
- **Keywords**: Intimidate -- CORRECT
- **Oracle text**: "Intimidate" -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
