## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.\n{1}, {T}: Add one mana of any color.
**Scryfall type line**: Land
**Status**: PASS

Implementation correctly models:
- Name, no mana cost (land), type Land
- Mana ability: {T} for {C}
- Activated ability: {1}, {T} for one mana of any color (implemented as 5 separate activated abilities, one per color)
- The 5-option approach is a reasonable workaround for the engine lacking a "choose a color" mechanism
- Tests: `shimmering_grotto_card_data` and `shimmering_grotto_taps_for_colorless` in innistrad_simple_cards.rs

No issues found.
