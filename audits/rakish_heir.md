# Audit: Rakish Heir

## Official Oracle
- **Name:** Rakish Heir
- **Cost:** {2}{R}
- **Type:** Creature — Vampire
- **Oracle Text:** Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
- **P/T:** 2/2

## Implementation Review
- **Name:** OK
- **Cost:** {2}{R} — OK
- **Type:** Creature, subtypes ["Vampire"] — OK
- **Oracle Text:** "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on that Vampire." — close match (official says "on it" not "on that Vampire", but semantically identical) — OK
- **P/T:** 2/2 — OK
- **Triggered Abilities:** AnyCombatDamageToPlayer — OK
- **on_any_combat_damage_to_player:** Checks source is a Vampire controlled by same controller, adds +1/+1 counter to the source — OK
- **Vampire check:** Checks both registry subtypes and instance subtypes — OK

## Issues
None found.

## Verdict: PASS
