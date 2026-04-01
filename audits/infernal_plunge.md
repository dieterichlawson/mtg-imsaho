## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Mana cost {R}: correct
- Card type Sorcery: correct
- Additional cost SacrificeCreature declared in card_data: correct
- Adds {R}{R}{R} to mana pool: correct
- ISSUE: The sacrifice happens at resolution time rather than at casting time. Oracle requires it as an additional cost to cast. The comment acknowledges this as a simplification. If the creature dies before resolution (e.g., from another spell on the stack), the spell would still resolve and add mana incorrectly.
- ISSUE: The creature to sacrifice is auto-picked (first found) rather than player choice
- Tests exist in tier8_cards.rs covering sacrifice and mana addition

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- ISSUE (minor, documented simplification): Sacrifice happens at resolution rather than as a casting cost. Auto-picks creature to sacrifice rather than player choice. Both are acknowledged simplifications in code comments.
