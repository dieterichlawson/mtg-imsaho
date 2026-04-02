# Audit: Voiceless Spirit

## Scryfall Reference
- **Name:** Voiceless Spirit
- **Cost:** {2}{W}
- **Type:** Creature — Spirit
- **Oracle:** Flying, first strike
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/voiceless_spirit.rs`
- Name: "Voiceless Spirit" -- MATCH
- Cost: {2}{W} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Spirit"] -- MATCH
- P/T: 2/1 -- MATCH
- Keywords: [Flying, FirstStrike] -- MATCH

## Verdict
**PASS** — Correctly implemented with both keywords.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying, first strike
**Mana cost**: {2}{W}
**Type line**: Creature — Spirit
**P/T**: 2/1
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Voiceless Spirit", cost {2}{W}, 2/1, type Creature — Spirit, keywords [Flying, FirstStrike], oracle text "Flying, first strike". Vanilla creature with keywords only, no behavior needed beyond card_data. All correct.
