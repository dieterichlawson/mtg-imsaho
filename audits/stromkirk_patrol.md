# Audit: Stromkirk Patrol

## Oracle (Scryfall)
- **Name:** Stromkirk Patrol
- **Cost:** {4}{B}
- **Type:** Creature -- Vampire Soldier
- **Oracle:** Whenever Stromkirk Patrol deals combat damage to a player, put a +1/+1 counter on it.
- **P/T:** 4/3

## Implementation: `mtg-engine/src/cards/stromkirk_patrol.rs`
- **Name:** Stromkirk Patrol ✅
- **Cost:** {4}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Vampire, Soldier ✅
- **P/T:** 4/3 ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** adds +1/+1 counter, checks zone ✅

## Verdict: PASS -- no issues found
