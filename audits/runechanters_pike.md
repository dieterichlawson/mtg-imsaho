## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}
**Scryfall type line**: Artifact — Equipment
**Mana cost**: {2}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}, type Artifact, subtype Equipment
- Grants first strike via ContinuousEffect::GrantKeyword
- Dynamic P/T bonus: counts instant and sorcery cards in controller's graveyard
- Equip {2} as activated ability (sorcery speed only)
- Equipment enters battlefield properly with `is_equipment = true`
- Tests: 3 tests in tier9_cards.rs covering card data, first strike grant, and equip ability

No issues found.
