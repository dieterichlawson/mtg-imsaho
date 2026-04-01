## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature has first strike.\nAs long as equipped creature is a Human, it gets +1/+1.\nEquip {1}
**Scryfall type line**: Artifact — Equipment
**Mana cost**: {2}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}, type Artifact, subtype Equipment
- Grants first strike to equipped creature
- Conditional +1/+1 bonus when equipped creature is a Human (via `update_effects`)
- Equip {1} at sorcery speed
- Equipment enters battlefield with `is_equipment = true`
- Tests: 3 tests in tier9_equipment.rs covering data, non-Human behavior, and Human bonus

No issues found.
