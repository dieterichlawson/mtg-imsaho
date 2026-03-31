# Audit: Bloodcrazed Neonate

## Oracle (Scryfall)
- **Name:** Bloodcrazed Neonate
- **Cost:** {1}{R}
- **Type:** Creature — Vampire
- **Oracle:** Bloodcrazed Neonate attacks each combat if able. Whenever Bloodcrazed Neonate deals combat damage to a player, put a +1/+1 counter on it.
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/bloodcrazed_neonate.rs`
- **Name:** Bloodcrazed Neonate ✅
- **Cost:** {1}{R} ✅
- **Type:** Creature ✅
- **Subtypes:** Vampire ✅
- **P/T:** 2/1 ✅
- **Force attack:** ContinuousEffect::ForceAttack with OnSelf scope ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** adds +1/+1 counter via `state.add_counters` ✅
- **Zone check:** checks self is on battlefield ✅

## Verdict: PASS — no issues found
