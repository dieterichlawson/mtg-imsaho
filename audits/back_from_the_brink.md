## Audit — 2026-04-01

**Scryfall Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Mana cost {4}{U}{U}: correct
- Card type Enchantment: correct
- sorcery_speed_only: true: correct

Issues found:
1. **Mana cost of ability is wrong**: The Oracle text says "pay its mana cost" (the exiled creature's mana cost), but the implementation uses a flat Generic(2) cost. This is acknowledged as a simplification, but it means you can create copies of expensive creatures for just {2}.
2. **No player choice for which creature to exile**: The implementation picks the first creature in the graveyard (`.find()`). The Oracle text implies the player chooses which creature card to exile (it's part of the cost).
3. **Exile should be part of cost, not effect**: The creature is exiled as part of paying the activation cost, not on resolution. The implementation exiles after creating the token copy.

Test exists in tier15_cards.rs.

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Scryfall type line**: Enchantment
**Status**: ISSUE

1. **Mana cost of ability is wrong** (back_from_the_brink.rs:58-59): Uses flat Generic(2) instead of the exiled creature's mana cost. Major gameplay impact.
2. **No player choice for creature to exile** (back_from_the_brink.rs:75-78): Uses `.find()` to pick first creature in graveyard instead of letting player choose.
3. **Exile should be part of cost, not effect**: Creature exiled after token creation instead of as part of activation cost.
