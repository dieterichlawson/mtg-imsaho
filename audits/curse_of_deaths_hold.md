## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
Creatures enchanted player controls get -1/-1.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

No issues found. Mana cost {3}{B}{B} correct. Subtypes ["Aura", "Curse"] correct. Continuous effect correctly uses ModifyPT with scope Global(CreatureFilter::AttachedPlayer). Resolves as curse via resolve_curse helper. Test exists for debuffing opponent creatures.
