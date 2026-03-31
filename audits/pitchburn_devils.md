# Audit: Pitchburn Devils

## Official Oracle
- **Name:** Pitchburn Devils
- **Cost:** {4}{R}
- **Type:** Creature — Devil
- **Oracle Text:** When Pitchburn Devils dies, it deals 3 damage to any target.
- **P/T:** 3/3

## Implementation Review
- **Name:** OK
- **Cost:** {4}{R} — OK
- **Type:** Creature, subtypes ["Devil"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 3/3 — OK
- **Triggered Abilities:** SelfDies trigger — OK
- **on_dies:** Presents target choice to controller with "any target" (all creatures + all players), PendingEffect::DealDamage { amount: 3 } — OK
- **Damage event:** DealDamage pending effect correctly emits NonCombatDamageDealt via the engine's resolution handler — OK

## Issues
None found.

## Verdict: PASS
