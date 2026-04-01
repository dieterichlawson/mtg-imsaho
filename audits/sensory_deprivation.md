## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature\nEnchanted creature gets -3/-0.
**Scryfall type line**: Enchantment — Aura
**Mana cost**: {U}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {U}, type Enchantment, subtype Aura
- Continuous effect: ModifyPT { power: -3, toughness: 0, scope: Attached }
- Target requirement: Creature
- Resolution uses `helpers::resolve_aura`
- Tests: `sensory_deprivation_reduces_power` in innistrad_cards.rs

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature. Enchanted creature gets -3/-0.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

No issues found. Correctly applies -3/-0 as continuous effect on attached creature.
