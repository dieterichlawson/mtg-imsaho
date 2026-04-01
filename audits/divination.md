## Audit — 2026-04-01

**Scryfall Oracle text**: Draw two cards.
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {2}{U}: correct.
- Type Sorcery: correct.
- Draws 2 cards via `draw_cards`: correct.
- Uses `move_spell_after_resolve`: correct.
- No targets: correct.
- Tests exist in `spells.rs` (`divination_draws_two`) and `fizzle.rs`.
