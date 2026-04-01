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
## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature has first strike. As long as equipped creature is a Human, it gets +1/+1. Equip {1}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

- **Human check misses tokens**: `mtg-engine/src/cards/sharpened_pitchfork.rs:15-18` — `update_effects` only checks `registry.card_data(o.card_id)` subtypes for "Human", but does not also check `obj.subtypes`. This means Human tokens (which have subtypes on the object, not in the registry) won't get the +1/+1 bonus. Should also check `state.get_object(creature_id).map(|o| o.subtypes.iter().any(|s| s == "Human"))`.
