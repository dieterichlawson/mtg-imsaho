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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying\nSturmgeist's power and toughness are each equal to the number of cards in your hand.\nWhenever this creature deals combat damage to a player, draw a card.
**Type line**: Creature — Spirit
**Mana Cost**: {3}{U}{U}
**P/T**: */*
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Sturmgeist", cost {3}{U}{U}, type Creature, subtype Spirit, keyword Flying. Base P/T stored as 0/0 with dynamic_pt correctly computing hand size for both power and toughness. Combat damage trigger draws a card via on_combat_damage_to_player. All correct.
