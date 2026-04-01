## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nAs an additional cost to cast Stitched Drake, exile a creature card from your graveyard.
**Scryfall type line**: Creature — Zombie Drake
**Status**: ISSUE

- Name: correct ("Stitched Drake")
- Cost: {1}{U}{U} -- correct
- Type: Creature -- correct
- Subtypes: Zombie, Drake -- correct
- P/T: 3/4 -- correct
- Keywords: Flying -- correct
- Additional cost: ExileCreaturesFromGraveyard(1) -- correct in card_data

**Issue: Exiling logic is duplicated in on_resolve.** The `additional_cost` field is set to `ExileCreaturesFromGraveyard(1)`, which should be handled by the engine when casting. However, `on_resolve` also manually searches the graveyard and exiles a creature card. This could lead to double-exiling if the engine processes the additional cost. Additionally, `on_resolve` manually moves the creature to the battlefield instead of letting the engine handle it (most creatures don't override `on_resolve` at all). If the engine already handles the additional cost, the exile logic in `on_resolve` is redundant and buggy.

- Tests exist in `tier11_cards.rs`

## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
**Scryfall type line**: Creature — Zombie Drake
**Status**: PASS

No issues found. Card data, cost, types, subtypes, P/T, keywords, and additional cost all correct.
