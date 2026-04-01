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
