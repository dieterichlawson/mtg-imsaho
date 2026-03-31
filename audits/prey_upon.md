# Audit: Prey Upon

## Official Oracle
- **Name:** Prey Upon
- **Cost:** {G}
- **Type:** Sorcery
- **Oracle Text:** Target creature you control fights target creature you don't control.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {G} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Target:** TwoTargets(CreatureWithFilter(YouControl), CreatureWithFilter(YouDontControl)) — OK
- **on_resolve:** Handles both target orderings, calls combat::fight — OK

## Issues
None found.

## Verdict: PASS
