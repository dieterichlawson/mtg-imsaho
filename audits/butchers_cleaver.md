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
