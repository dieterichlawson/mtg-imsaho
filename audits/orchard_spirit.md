# Audit: Orchard Spirit

## Official Oracle
- **Name:** Orchard Spirit
- **Cost:** {2}{G}
- **Type:** Creature — Spirit
- **Oracle:** Orchard Spirit can't be blocked except by creatures with flying or reach.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/orchard_spirit.rs`
- **Name:** Orchard Spirit -- CORRECT
- **Cost:** {2}{G} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Spirit -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Continuous effect:** BlockRestriction, allowed_blockers: Or(HasKeyword(Flying), HasKeyword(Reach)), scope OnSelf -- CORRECT

## Verdict
**PASS** -- No issues found.
