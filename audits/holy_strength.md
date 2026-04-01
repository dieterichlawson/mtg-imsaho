## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Enchanted creature gets +1/+2.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

- Mana cost {W}: correct
- Card type Enchantment, subtype Aura: correct
- Target requirement Creature: correct
- Continuous effect ModifyPT +1/+2 with scope Attached: correct
- Resolves as aura: correct
- Tests exist in enchantments.rs covering creature buffing
