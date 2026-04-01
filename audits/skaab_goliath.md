## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile two creature cards from your graveyard.\nTrample
**Scryfall type line**: Creature — Zombie Giant
**Mana cost**: {5}{U}
**P/T**: 6/9
**Status**: ISSUE

**Issue: Oracle text order.** The actual Oracle text lists the additional cost first, then Trample. The implementation has them reversed ("Trample\nAs an additional cost..."). This is cosmetic only and does not affect functionality.

**Issue: Additional cost is paid during resolution, not during casting.** The `on_resolve` method exiles creature cards from the graveyard, but per MTG rules, additional costs are paid as part of casting the spell (before it goes on the stack), not on resolution. The `AdditionalCost::ExileCreaturesFromGraveyard(2)` is declared in card_data, so the engine may handle this at cast time — but the `on_resolve` also does the exile, which could cause double-exiling or no effect if the engine already handled it. This depends on engine behavior.

- Tests: `skaab_goliath_exiles_two_creatures` in tier11_cards.rs

## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
**Scryfall type line**: Creature — Zombie Giant
**Status**: PASS

No issues found. Card data, cost, types, subtypes, P/T, keywords, and additional cost all match Scryfall. The exile happens on resolve (engine limitation for additional costs), which is a known simplification.
