# Audit: Ranger's Guile

## Official Oracle
- **Name:** Ranger's Guile
- **Cost:** {G}
- **Type:** Instant
- **Oracle Text:** Target creature you control gets +1/+1 and gains hexproof until end of turn.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {G} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Target:** CreatureWithFilter(YouControl) — OK
- **is_valid_target:** Checks battlefield, is creature, controller matches — OK
- **on_resolve:** Applies +1/+1 UntilEndOfTurnEffect and Hexproof UntilEndOfTurnKeyword — OK

## Issues
None found.

## Verdict: PASS
