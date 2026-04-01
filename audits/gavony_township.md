## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
**Scryfall type line**: Land
**Status**: PASS

- Card type Land (no mana cost): correct
- Mana ability {T}: Add {C}: correct
- Activated ability {2}{G}{W}, {T}: correct cost, requires tap
- On activation, puts a +1/+1 counter on each creature controller controls: correct
- Tests exist in tier10_cards.rs covering card data and counter placement
