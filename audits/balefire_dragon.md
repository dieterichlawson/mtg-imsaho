# Audit: Balefire Dragon

## Oracle (Scryfall)
- **Name:** Balefire Dragon
- **Cost:** {5}{R}{R}
- **Type:** Creature — Dragon
- **Oracle:** Flying. Whenever Balefire Dragon deals combat damage to a player, it deals that much damage to each creature that player controls.
- **P/T:** 6/6

## Implementation: `mtg-engine/src/cards/balefire_dragon.rs`
- **Name:** Balefire Dragon ✅
- **Cost:** {5}{R}{R} ✅
- **Type:** Creature ✅
- **Subtypes:** Dragon ✅
- **P/T:** 6/6 ✅
- **Keywords:** Flying ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** deals `amount` damage to each creature damaged player controls ✅
- **Damage is non-combat:** Uses `damage_marked` + `NonCombatDamageDealt` event ✅
- **damaged_by tracking:** pushes `self_id` ✅
- **Zone check:** checks self is on battlefield ✅

## Verdict: PASS — no issues found
