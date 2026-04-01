## Audit — 2026-04-01

**Scryfall Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Scryfall type line**: Creature — Wurm
**Status**: PASS

- Mana cost {1}{G}: correct
- Base P/T 0/0 (represented as */*): correct — the implementation uses Some(0)/Some(0) as base with dynamic_pt override
- Subtype Wurm: correct
- dynamic_pt counts creature cards in controller's graveyard: correct
- Uses o.power.is_some() to identify creature cards: reasonable proxy
- Test exists in tier7_cards.rs

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Scryfall type line**: Creature — Wurm
**Status**: PASS

No issues found. dynamic_pt correctly counts creature cards in controller's graveyard.
