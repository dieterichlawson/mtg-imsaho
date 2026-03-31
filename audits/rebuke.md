# Audit: Rebuke

## Official Oracle
- **Name:** Rebuke
- **Cost:** {2}{W}
- **Type:** Instant
- **Oracle Text:** Destroy target attacking creature.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{W} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Target:** CreatureWithFilter(Attacking) — OK
- **is_valid_target:** Checks battlefield, is creature, is in combat.attackers — OK
- **on_resolve:** Uses resolve_destroy helper — OK

## Issues
None found.

## Verdict: PASS
