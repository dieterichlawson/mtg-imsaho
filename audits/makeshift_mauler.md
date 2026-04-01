## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard.
**Scryfall type line**: Creature — Zombie Horror
**Status**: ISSUE

**Findings**:

1. Name: Makeshift Mauler -- correct
2. Cost: {3}{U} -- correct
3. Type: Creature -- correct
4. Subtypes: Zombie, Horror -- correct
5. P/T: 4/5 -- correct
6. Additional cost: exile a creature card from graveyard -- declared in card_data as `AdditionalCost::ExileCreaturesFromGraveyard(1)` which is correct.
7. **ISSUE — Double exile in on_resolve**: The `on_resolve` method manually exiles a creature card from the graveyard again during resolution. If the engine already handles the `AdditionalCost` at cast time, this would exile TWO creature cards total. If the engine does NOT handle it automatically, then only the on_resolve exile happens (at the wrong time -- should be at cast time, not resolution). Either way, one of these is redundant or incorrectly timed.
8. Tests exist in tier11_cards.rs.

**Summary**: The additional cost exile logic may be duplicated -- once via the `additional_cost` field and once manually in `on_resolve`. This needs investigation to determine which path the engine actually uses.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard.
**Scryfall type line**: Creature -- Zombie Horror
**Status**: ISSUE

- Confirmed potential double-exile issue. The additional_cost field declares ExileCreaturesFromGraveyard(1), AND the on_resolve manually exiles a creature from graveyard. If the engine processes the additional_cost at cast time, the creature ends up exiling TWO cards. If the engine ignores additional_cost for this type, only the resolution exile happens (wrong timing but functionally close).
- All other card data (cost, types, subtypes, P/T) is correct.
