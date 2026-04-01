## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Each player mills a card.
**Scryfall type line**: Artifact
**Status**: PASS

- Mana cost {1}: correct
- Card type Artifact: correct
- Activated ability {T}: correct (requires tap, no mana cost)
- On activation: mills 1 card for each player: correct
- Tests exist in innistrad_simple_cards.rs covering card data and mill functionality
