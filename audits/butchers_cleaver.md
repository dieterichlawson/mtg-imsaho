## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature gets +3/+0.
As long as equipped creature is a Human, it has lifelink.
Equip {3}
**Scryfall type line**: Artifact — Equipment
**Status**: PASS

- Mana cost {3}: correct
- Card types Artifact, subtypes Equipment: correct
- +3/+0 to equipped creature: correct
- Conditional lifelink for Humans: correct (via update_effects)
- Equip {3}, sorcery speed: correct
- is_valid_target restricts equip to own creatures: correct
- on_resolve sets is_equipment: correct
- Tests exist in tier9_equipment.rs covering data, non-human (power only), and human (power + lifelink)

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

1. **Human check only uses registry, not object subtypes** (butchers_cleaver.rs:15-18): The `update_effects` method only checks `registry.card_data(o.card_id)` for Human subtype but does not check `o.subtypes` on the object. Human tokens (which store subtypes on the object) would not get lifelink.
