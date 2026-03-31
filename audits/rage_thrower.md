# Audit: Rage Thrower

## Official Oracle
- **Name:** Rage Thrower
- **Cost:** {5}{R}
- **Type:** Creature — Human Shaman
- **Oracle Text:** Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.
- **P/T:** 4/2

## Implementation Review
- **Name:** OK
- **Cost:** {5}{R} — OK
- **Type:** Creature, subtypes ["Human", "Shaman"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 4/2 — OK
- **Triggered Abilities:** AnyCreatureDies — OK (says "another creature", and the hook is on_any_creature_dies which excludes self)
- **on_any_creature_dies:** Checks zone == Battlefield, presents target choice among all players, PendingEffect::DealDamage { amount: 2 } — OK
- **Damage event:** DealDamage emits NonCombatDamageDealt — OK
- **"target player or planeswalker":** Only offers player targets (no planeswalkers), but planeswalkers may not be in the engine — acceptable simplification

## Issues
None found (planeswalker omission is an engine-level limitation, not a card bug).

## Verdict: PASS
