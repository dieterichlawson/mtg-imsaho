## Audit — 2026-04-01

**Scryfall Oracle text**: Hexproof
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
**Scryfall type line**: Legendary Creature — Spirit Cleric
**Status**: PASS

- Mana cost {1}{W}{U}: correct
- 2/2 stats: correct
- Supertype Legendary: correct
- Subtypes Spirit Cleric: correct
- Keyword Hexproof: correct
- on_attacks creates 4/4 white Angel token with flying, sets tapped=true, adds to combat attackers: correct
- on_end_combat exiles the angel token: correct
- Angel token tracking via card_state: reasonable approach
- Tests exist in tier15_cards.rs
