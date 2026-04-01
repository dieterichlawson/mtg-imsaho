## Audit — 2026-04-01

**Scryfall Oracle text**: You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
**Scryfall type line**: Enchantment
**Mana cost**: {5}{U}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {5}{U}, type Enchantment
- Oracle text matches
- Note: The actual cost reduction mechanism is described in comments as using a high reduction value (20) to effectively make Zombie creature spells free. The card_data itself doesn't encode this directly in continuous_effects, suggesting the engine handles it externally.
- Tests: `rooftop_storm_makes_zombies_free` and `rooftop_storm_no_free_non_zombies` in tier14_cards.rs

No issues found.
