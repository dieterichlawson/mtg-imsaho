## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature gets +1/+2 and has hexproof.\nEquip {3}
**Scryfall type line**: Artifact — Equipment
**Status**: PASS

- Name: Mask of Avacyn -- correct
- Cost: {2} -- correct
- Type: Artifact -- correct
- Subtypes: Equipment -- correct
- Continuous effects: +1/+2 and hexproof to equipped creature (Attached scope) -- correct
- Equip cost: {3} -- correct
- Equip is sorcery speed -- correct
- Target validation: own creatures only -- correct
- Tests exist in tier9_equipment.rs

No issues found. Implementation matches Oracle text.
