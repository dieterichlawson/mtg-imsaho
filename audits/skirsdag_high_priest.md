# Audit: Skirsdag High Priest

## Oracle (Scryfall)
- **Name:** Skirsdag High Priest
- **Cost:** {1}{B}
- **Type:** Creature -- Human Cleric
- **Oracle:** Morbid -- {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/skirsdag_high_priest.rs`
- **Name:** Skirsdag High Priest ✅
- **Cost:** {1}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Cleric ✅
- **P/T:** 1/2 ✅
- **Morbid check:** checks `state.creature_died_this_turn` ✅
- **Tap cost:** requires_tap: true, plus taps 2 other untapped creatures ✅
- **Sorcery speed:** sorcery_speed_only: true ✅
- **Token:** 5/5 black Demon with flying, subtypes ["Demon"] ✅
- **Pre-checks:** verifies not tapped, not summoning sick, at least 2 other untapped creatures ✅

### Note
- The two creatures to tap are auto-selected (first two found) rather than player-chosen. This is a simplification but functionally acceptable in most cases.

## Verdict: PASS -- no issues found
