## Audit — 2026-04-01

**Scryfall Oracle text**: Hexproof
Invisible Stalker can't be blocked.
**Scryfall type line**: Creature — Human Rogue
**Status**: PASS

- Mana cost {1}{U}: correct
- 1/1 stats: correct
- Subtypes Human Rogue: correct
- Keyword Hexproof: correct
- CantBeBlocked continuous effect (EffectScope::OnSelf): correct
- Tests exist in innistrad_cards.rs and card_mechanics.rs covering hexproof and unblockable

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Hexproof. Invisible Stalker can't be blocked.
**Scryfall type line**: Creature -- Human Rogue
**Status**: PASS

No issues found.
