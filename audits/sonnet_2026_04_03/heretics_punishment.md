## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Choose any target" includes both permanents and players: PASS
- Target legality check causes entire ability to fizzle if target becomes illegal: PASS  
- Mill count respects library size (mills all remaining if fewer than 3): PASS
- No damage dealt when all milled cards have mana value 0: PASS
- Double-faced cards in graveyard use front face mana value: PASS
- Damage tracking generates correct NonCombatDamageDealt events: PASS
- Damage properly applied to creatures (damage_marked) and players (life): PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic mill-then-damage functionality: `tier15_cards.rs:316` (heretics_punishment_mills_then_deals_damage)
- Damage tracking on creatures: `tier15_cards.rs:354` (heretics_punishment_tracks_damaged_by_on_creature)
- Target fizzling prevents entire ability: `tier15_cards.rs:383` (heretics_punishment_fizzles_when_target_illegal)
- Mill count with fewer than 3 cards in library: NOT TESTED
- No damage when all milled cards have mana value 0: NOT TESTED
- Double-faced cards use front face mana value when milled: NOT TESTED
- Player targeting and life loss: NOT TESTED

Sources:
- [Heretic's Punishment rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Heretic-s-Punishment/rulings/)
- [Heretic's Punishment | Innistrad | Modern | Card Kingdom](https://www.cardkingdom.com/mtg/innistrad/heretics-punishment)
- [Heretic's Punishment • Enchantment (Innistrad) - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Heretic-s-Punishment/)