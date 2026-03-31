# Audit: Reaper from the Abyss

## Official Oracle
- **Name:** Reaper from the Abyss
- **Cost:** {3}{B}{B}{B}
- **Type:** Creature — Demon
- **Oracle Text:** Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
- **P/T:** 6/6

## Implementation Review
- **Name:** OK
- **Cost:** {3}{B}{B}{B} — OK
- **Type:** Creature, subtypes ["Demon"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 6/6 — OK
- **Keywords:** Flying — OK
- **Triggered Abilities:** EndStep trigger — OK
- **on_end_step:** Checks creature_died_this_turn (morbid), filters non-Demon creatures, presents target choice, PendingEffect::DestroyCreature — OK
- **Non-Demon filter:** Checks both registry and instance subtypes — OK

## Issues
None found.

## Verdict: PASS
