# Audit: Laboratory Maniac

## Oracle (Official)
- **Name:** Laboratory Maniac
- **Cost:** {2}{U}
- **Type:** Creature — Human Wizard
- **Oracle:** If you would draw a card while your library has no cards in it, you win the game instead.
- **P/T:** 2/2

## Implementation
- Name: "Laboratory Maniac" -- CORRECT
- Cost: {2}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Human", "Wizard"] -- CORRECT
- P/T: 2/2 -- CORRECT
- Oracle text matches -- CORRECT

## Issues
1. **ISSUE (major):** The replacement effect "if you would draw a card while your library has no cards in it, you win the game instead" has NO implementation beyond the oracle text string. There is no `on_draw` hook or replacement effect logic. The card is just a vanilla 2/2 in practice.

## Verdict: FAIL — win-the-game replacement effect not implemented
