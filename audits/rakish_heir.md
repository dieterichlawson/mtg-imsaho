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

---

# Audit: Rakish Heir (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rakish Heir
- **Mana Cost:** {2}{R}
- **Type:** Creature — Vampire
- **P/T:** 2/2
- **Oracle Text:** Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.

## Card Data Verification
- **Name:** Correct ("Rakish Heir")
- **Cost:** Correct ({2}{R})
- **Type:** Correct (Creature)
- **Subtypes:** Correct (Vampire)
- **P/T:** Correct (2/2)
- **Keywords:** Correct (none)

## Behavior Verification
- **Trigger:** Correct — `AnyCombatDamageToPlayer` hook fires for any combat damage to a player.
- **Vampire check:** Correct — verifies the damage source is a Vampire controlled by the same controller as Rakish Heir.
- **Counter placement:** Correct — places +1/+1 counter on the source creature (the Vampire that dealt damage) via `state.add_counters(source_id, CounterType::PlusOnePlusOne, 1)`.

## Result: PASS
