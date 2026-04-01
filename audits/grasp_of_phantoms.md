## Audit — 2026-04-01

**Scryfall Oracle text**: Put target creature on top of its owner's library.
Flashback {7}{U}
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Mana cost {3}{U}: correct
- Card type Sorcery: correct
- Target requirement Creature: correct
- Flashback cost {7}{U}: correct
- On resolve: moves creature to Library zone and inserts at position 0 (top): correct
- ISSUE: Potential double-insert — move_object likely already adds the card to the library zone, then the code manually inserts at position 0. This could result in the card appearing twice in library_order if move_object appends to the end of library_order. The insert at position 0 is correct intent but may cause duplication depending on move_object implementation.
- Tests exist in tier11_cards.rs covering top-of-library placement and flashback
