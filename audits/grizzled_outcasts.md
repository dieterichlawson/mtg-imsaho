# Audit: Grizzled Outcasts / Krallenhorde Wantons

## Oracle Reference (Scryfall)
**Front Face: Grizzled Outcasts**
- Cost: {4}{G}
- Type: Creature -- Human Werewolf
- P/T: 4/4
- Oracle: "At the beginning of each upkeep, if no spells were cast last turn, transform Grizzled Outcasts."

**Back Face: Krallenhorde Wantons**
- Type: Creature -- Werewolf
- P/T: 7/7
- Oracle: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Krallenhorde Wantons."

## Implementation: grizzled_outcasts.rs

## Issues Found

No issues found. Front and back face names, types, subtypes, P/T values, and oracle text all match. Werewolf transform logic is correct (same pattern as other werewolves).

## Verdict: PASS
