# Audit: Moment of Heroism

## Official Oracle
- **Name:** Moment of Heroism
- **Cost:** {1}{W}
- **Type:** Instant
- **Oracle:** Target creature gets +2/+2 and gains lifelink until end of turn.

## Implementation: `mtg-engine/src/cards/moment_of_heroism.rs`
- **Name:** Moment of Heroism -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Type:** Instant -- CORRECT
- **Target:** Creature -- CORRECT
- **on_resolve:** +2/+2 via UntilEndOfTurnEffect, lifelink via UntilEndOfTurnKeyword -- CORRECT

## Verdict
**PASS** -- No issues found.
