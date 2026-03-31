# Audit: Nightbird's Clutches

## Official Oracle
- **Name:** Nightbird's Clutches
- **Cost:** {1}{R}
- **Type:** Sorcery
- **Oracle:** Up to two target creatures can't block this turn. Flashback {3}{R}

## Implementation: `mtg-engine/src/cards/nightbirds_clutches.rs`
- **Name:** Nightbird's Clutches -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {3}{R} -- CORRECT
- **Target:** UpToTargets(2, Creature) -- CORRECT
- **on_resolve:** Adds targets to until_end_of_turn_cant_block -- CORRECT

## Verdict
**PASS** -- No issues found.
