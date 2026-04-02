# Audit: Night Revelers

## Official Oracle
- **Name:** Night Revelers
- **Cost:** {4}{R}
- **Type:** Creature — Vampire
- **Oracle:** Night Revelers has haste as long as an opponent controls a Human.
- **P/T:** 4/4

## Implementation: `mtg-engine/src/cards/night_revelers.rs`
- **Name:** Night Revelers -- CORRECT
- **Cost:** {4}{R} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Vampire -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Continuous effect:** ConditionalKeyword Haste, condition OpponentControlsSubtype("Human"), scope OnSelf -- CORRECT

## Verdict
**PASS** -- No issues found.

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Night Revelers  
**Type**: Creature — Vampire | **Cost**: {4}{R} | **P/T**: 4/4  
**Oracle text**: "This creature has haste as long as an opponent controls a Human."

### Checks
- Name: "Night Revelers" -- PASS
- Cost: {4}{R} -- PASS
- Type: Creature -- PASS
- Subtypes: Vampire -- PASS
- P/T: 4/4 -- PASS
- Oracle text string: ISSUE
  - **Oracle**: "This creature has haste as long as an opponent controls a Human."
  - **Code**: "Night Revelers has haste as long as an opponent controls a Human."
  - Oracle uses "This creature" (modern templating) while the code uses the card's name.
- Behavior: ConditionalKeyword with Haste, condition OpponentControlsSubtype("Human"), scope OnSelf -- PASS (correctly models the ability)

**Verdict: ISSUE** — oracle_text field says "Night Revelers has haste" instead of current oracle "This creature has haste"
