## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nOther Spirit creatures you control get +0/+1.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

- Mana cost {4}{W}: correct.
- Type Creature, subtype Spirit: correct.
- Power/Toughness 3/3: correct.
- Keywords: Flying: correct.
- Continuous effect: Other Spirits you control get +0/+1 via `GlobalOther(And(You, HasSubtype("Spirit")))`: correct. Uses `GlobalOther` (not `Global`) so it excludes itself.
- Tests exist in `tier5_cards.rs` (`gallows_warden_buffs_other_spirits`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. Other Spirit creatures you control get +0/+1.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

No issues found. Correctly uses GlobalOther scope for the +0/+1 buff.
