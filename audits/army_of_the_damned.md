## Audit — 2026-04-01

**Scryfall Oracle text**: Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped.
Flashback {7}{B}{B}{B}
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {5}{B}{B}{B}: correct
- Card type Sorcery: correct
- Flashback {7}{B}{B}{B}: correct
- Creates 13 tokens in a loop: correct
- Tokens are 2/2 black Zombie creature tokens: correct
- Tokens enter tapped (obj.tapped = true): correct
- Uses move_spell_after_resolve: correct
- Test exists in tier12_cards.rs
