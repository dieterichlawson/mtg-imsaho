## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues
- Oracle text field missing "Enchant creature" prefix (mtg-engine/src/cards/isd/ghostly_possession.rs:24)
  - Oracle text says: `Enchant creature\nEnchanted creature has flying.\nPrevent all combat damage that would be dealt to and dealt by enchanted creature.`
  - Code does: `oracle_text: "Enchanted creature has flying. Prevent all combat damage that would be dealt to and dealt by enchanted creature.".into(),` (missing "Enchant creature" prefix)

### Tricky interactions checked
- Flying keyword grant via GrantKeyword continuous effect: pass
- Combat damage prevention both to and from enchanted creature: pass
- Aura attachment targeting via TargetRequirement::Creature: pass
- EffectScope::Attached correctly limits effects to attached creature only: pass
- Continuous effect evaluation (re-applied while attached): pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flying keyword grant: `mtg-engine/tests/innistrad_cards.rs:372-384`
- Combat damage prevention to enchanted creature: `mtg-engine/tests/card_mechanics.rs:276-301`
- Combat damage prevention from enchanted creature: `mtg-engine/tests/card_mechanics.rs:276-301`
- Aura attachment mechanics: NOT TESTED
- Effect removal when aura leaves battlefield: NOT TESTED