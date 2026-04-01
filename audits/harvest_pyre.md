## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard.
Harvest Pyre deals damage to target creature equal to the number of cards exiled this way.
**Scryfall type line**: Instant
**Status**: ISSUE

- Mana cost {1}{R}: correct
- Card type Instant: correct
- Target requirement Creature: correct
- ISSUE: The additional cost (exile cards from graveyard) is handled at resolution time rather than at casting time. Oracle requires this as an additional cost to cast, meaning cards should be exiled when the spell is put on the stack. The implementation exiles on resolution.
- ISSUE: The implementation always exiles ALL graveyard cards for maximum damage. Oracle says "any number" — the player should choose how many to exile. This is noted as a known limitation.
- Damage dealing: correctly deals damage equal to exiled count to target creature
- Tests exist in tier8_cards.rs covering damage calculation, empty graveyard case, and own-graveyard-only restriction

## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.
**Scryfall type line**: Instant
**Status**: ISSUE

1. **Auto-exiles all graveyard cards**: Oracle says "exile X cards" — the player chooses X. The code auto-exiles ALL cards from the graveyard for maximum damage. The player should choose how many cards to exile. (Line 44-47 in harvest_pyre.rs) Acknowledged in code comments.
2. **Oracle text in code says "any number" instead of "X"**: The code's oracle_text field says "exile any number of cards" but the actual Oracle says "exile X cards." (Line 25)
3. **Additional cost not enforced during casting**: The exile is done during resolution, not as an additional cost during casting. This means the number of cards available could change between cast and resolution. The Oracle specifies this is an additional cost (paid during casting). (Line 29)
