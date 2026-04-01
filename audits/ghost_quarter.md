## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.
{T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Scryfall type line**: Land
**Status**: PASS

- Card type Land (no mana cost): correct
- Mana ability {T}: Add {C}: correct
- Activated ability: requires tap, sacrifice this, targets a land: correct
- On activation: destroys target land, then searches controller's library for a basic land (checking card_types Land + supertypes Basic): correct
- Basic land enters untapped: correct (summoning_sick set to false, but lands don't have summoning sickness — harmless)
- Auto-searches for first basic land rather than player choice: simplification noted
- Tests exist in innistrad_simple_cards.rs covering card data and mana tap
