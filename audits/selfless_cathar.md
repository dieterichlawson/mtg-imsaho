## Audit — 2026-04-01

**Scryfall Oracle text**: {1}{W}, Sacrifice Selfless Cathar: Creatures you control get +1/+1 until end of turn.
**Scryfall type line**: Creature — Human
**Mana cost**: {W}
**P/T**: 1/1
**Status**: ISSUE

**Issue: Subtype mismatch.** Implementation has subtypes `["Human", "Cleric"]` but Oracle type line is "Creature — Human" (no Cleric subtype). Selfless Cathar is not a Cleric.

The ability implementation is correct:
- Activated ability with {1}{W} cost and SacrificeThis
- Grants +1/+1 until end of turn to all creatures you control
- Tests: `selfless_cathar_pump_all_creatures` in tier8_cards.rs
## Audit — 2026-04-01

**Scryfall Oracle text**: {1}{W}, Sacrifice Selfless Cathar: Creatures you control get +1/+1 until end of turn.
**Scryfall type line**: Creature — Human Cleric
**Status**: PASS

No issues found. Activated ability with sacrifice cost correctly implemented.
