## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.\nFlying\nYou may cast Skaab Ruinator from your graveyard.
**Scryfall type line**: Creature — Zombie Horror
**Mana cost**: {1}{U}{U}
**P/T**: 5/6
**Status**: ISSUE

**Issue: Oracle text order.** The actual Oracle text lists the additional cost first, then Flying, then the graveyard-cast ability. The implementation has "Flying\nAs an additional cost...\nYou may cast..." which puts Flying first. Cosmetic only.

**Same concern as Skaab Goliath**: The `on_resolve` method manually exiles creatures from the graveyard, but `AdditionalCost::ExileCreaturesFromGraveyard(3)` is also declared in card_data. Potential double-exile depending on engine handling.

**Positive**: `can_cast_from_graveyard` returns true, correctly modeling the "You may cast Skaab Ruinator from your graveyard" ability.

- Tests: `skaab_ruinator_exiles_creatures_from_graveyard` in tier15_cards.rs

## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Scryfall type line**: Creature — Zombie Horror
**Status**: PASS

No issues found. Card data, cost, types, subtypes, P/T, keywords, additional cost, and `can_cast_from_graveyard` all match Scryfall. Self-exclusion from exile candidates is correctly implemented.
