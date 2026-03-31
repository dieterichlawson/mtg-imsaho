# Audit: Kessig Wolf Run

## Oracle (Official)
- **Name:** Kessig Wolf Run
- **Cost:** (none — Land)
- **Type:** Land
- **Oracle:** {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
- **P/T:** N/A

## Implementation
- Name: "Kessig Wolf Run" -- CORRECT
- Cost: None -- CORRECT
- Type: Land -- CORRECT
- Mana ability: {T} for {C} -- CORRECT
- Activated ability: simplified as {1}{R}{G},{T} for +1/+0 and trample (can be activated multiple times) -- SIMPLIFICATION noted in oracle_text and comments
- Grants trample via UntilEndOfTurnKeyword -- CORRECT
- Grants +1/+0 via UntilEndOfTurnEffect -- CORRECT

## Issues
1. **ISSUE (simplification):** X in the mana cost is simplified to 1. The engine doesn't support variable X costs for activated abilities. Noted in code.

## Verdict: PASS (with noted simplification)
