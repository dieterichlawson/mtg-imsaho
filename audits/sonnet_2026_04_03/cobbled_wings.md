## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Continuous effect evaluation: PASS - The flying grant uses `ContinuousEffect::GrantKeyword` with `EffectScope::Attached`, which properly evaluates continuously through the engine's `has_keyword` → `has_continuous_effect` → `effect_applies_to` chain.
- Sorcery speed restriction: PASS - The equip ability correctly sets `sorcery_speed_only: true` and the engine properly enforces this restriction in `legal_actions` by checking `!is_sorcery_speed`.
- Targeting restriction: PASS - The equip ability correctly targets only creatures you control via `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` and validates this in `is_valid_target`.
- Attachment mechanism: PASS - The `on_activate_ability` method correctly sets `obj.attached_to = Some(*creature_id)` to establish the equipment-creature link.
- Equipment detachment on creature death: PASS - Verified in tests that equipment properly detaches when the equipped creature dies.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic card data verification: `tier9_equipment.rs:54-64` / TESTED
- Equipment enters battlefield as equipment: `tier9_equipment.rs:67-77` / TESTED  
- Flying grant to attached creature: `tier9_equipment.rs:80-96` / TESTED
- Equip targeting restriction (only your creatures): `tier9_equipment.rs:99-116` / TESTED
- Equipment detachment when creature dies: `tier9_equipment.rs:398-418` / TESTED
- Equipment re-attachment to different creatures: `tier9_equipment.rs:421-440` / TESTED
- Full cast-and-equip flow: `tier9_equipment.rs:443-463` / TESTED
- Sorcery speed timing restriction: NOT DIRECTLY TESTED (verified through engine code review)

Sources:
- [Cobbled Wings · Innistrad (ISD) #219 - Scryfall](https://scryfall.com/card/isd/219/cobbled-wings)
- [Cobbled Wings rulings - MTG Assist](https://www.mtgassist.com/cards/Ixalan/Cobbled-Wings/rulings/)