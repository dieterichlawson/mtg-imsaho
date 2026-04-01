## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Enchanted creature has intimidate.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

- Mana cost {B}: correct
- Card type Enchantment, subtype Aura: correct
- Target requirement Creature: correct
- Grants Intimidate via continuous effect (EffectScope::Attached): correct
- Resolves as aura: correct
- Tests exist in innistrad_cards.rs covering intimidate grant
