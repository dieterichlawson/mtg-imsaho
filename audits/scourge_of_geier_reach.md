# Audit: Scourge of Geier Reach

## Official Oracle
- **Name:** Scourge of Geier Reach
- **Cost:** {3}{R}{R}
- **Type:** Creature — Elemental
- **Oracle Text:** Scourge of Geier Reach gets +1/+1 for each creature your opponents control.
- **P/T:** 3/3

## Implementation Review
- **Name:** OK
- **Cost:** {3}{R}{R} — OK
- **Type:** Creature, subtypes ["Elemental"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 3/3 base — OK
- **dynamic_pt:** Returns (3 + opponent_creatures, 3 + opponent_creatures) — ISSUE

## Issues
None found. Verified that the engine's effective_power/effective_toughness uses dynamic_pt as a REPLACEMENT for base P/T (not additive), so returning (3 + N, 3 + N) is correct.

## Verdict: PASS
