## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nSturmgeist's power and toughness are each equal to the number of cards in your hand.\nWhenever Sturmgeist deals combat damage to a player, draw a card.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

- Name: correct ("Sturmgeist")
- Cost: {3}{U}{U} -- correct
- Type: Creature -- correct
- Subtypes: Spirit -- correct
- P/T: */* (base 0/0) -- correct
- Keywords: Flying -- correct
- `dynamic_pt` returns hand size for both power and toughness -- correct
- `on_combat_damage_to_player` draws 1 card -- correct
- Tests exist in `tier6_cards.rs`
- No issues found
