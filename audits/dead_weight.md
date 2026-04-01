## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Enchanted creature gets -2/-2.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

No issues found. Mana cost {B} correct. Subtypes ["Aura"] correct. Continuous effect ModifyPT -2/-2 with Attached scope. Resolves via resolve_aura helper. Test exists verifying it kills a 2-toughness creature.
