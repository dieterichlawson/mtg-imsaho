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
