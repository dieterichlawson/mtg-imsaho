## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Scryfall type line**: Enchantment — Aura
**Status**: ISSUE

- Mana cost {1}{W}: correct
- Card type Enchantment, subtype Aura: correct
- Target requirement: Creature: correct
- Uses resolve_aura helper: correct
- Human check grants +2/+2: correct
- Non-Human check prevents attack and block: correct

Issues found:
1. **Continuous effect is not dynamic**: The implementation checks whether the creature is Human at ETB time (on_enter_battlefield) and locks in the effect. If the creature's type changes later (e.g., gains or loses the Human subtype via another effect), the Bonds of Faith effect won't update. The Oracle text says "as long as it's a Human" which should be checked continuously. This is a moderate fidelity issue.

Tests exist in bug_fixes.rs and card_mechanics.rs.

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Scryfall type line**: Enchantment — Aura
**Status**: ISSUE

1. **Human check is not continuous** (bonds_of_faith.rs:39-69): Effect is locked at ETB time. If creature type changes, the effect won't update. Oracle says "as long as it's a Human" which should be checked continuously.
2. **Token Human check incomplete** (bonds_of_faith.rs:43-46): Only checks registry card data for Human subtype, does not also check `o.subtypes` on the object. Human tokens (which have subtypes set on the object, not in the registry) would not be detected as Humans. Should check both `registry.card_data(o.card_id)` and `o.subtypes`.
