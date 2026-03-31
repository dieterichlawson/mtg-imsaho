# Audit: Sturmgeist

## Oracle (Scryfall)
- **Name:** Sturmgeist
- **Cost:** {3}{U}{U}
- **Type:** Creature -- Spirit
- **Oracle:** Flying. Sturmgeist's power and toughness are each equal to the number of cards in your hand. Whenever Sturmgeist deals combat damage to a player, draw a card.
- **P/T:** */*

## Implementation: `mtg-engine/src/cards/sturmgeist.rs`
- **Name:** Sturmgeist ✅
- **Cost:** {3}{U}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Spirit ✅
- **Base P/T:** 0/0 ✅
- **Keywords:** Flying ✅
- **dynamic_pt:** counts cards in controller's hand ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** draws 1 card, checks zone ✅

## Verdict: PASS -- no issues found
