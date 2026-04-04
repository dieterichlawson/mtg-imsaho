## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +2/+0.\nAs long as equipped creature is a Human, it gets an additional +1/+0.\nEquip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- "As long as" Human condition is evaluated once at equip time and never re-evaluated
  - Oracle text says: `"As long as equipped creature is a Human, it gets an additional +1/+0."`
  - Code does: `update_effects()` (silver_inlaid_dagger.rs lines 15–29) is called only from `on_activate_ability()` (line 86). It sets `instance_continuous_effects` once, based on the Human check at equip resolution time. If the equipped creature later loses the Human subtype (e.g., a Human werewolf such as Villagers of Estwald transforms to Howlpack of Estwald), `instance_continuous_effects` is never re-computed. The creature continues to receive +3/+0 instead of +2/+0. The `continuous_pt_mods` engine path (state.rs line 735) reads `instance_continuous_effects` directly without re-evaluating any Human condition.

- `update_effects()` does not check `o.subtypes` when detecting Human subtype, missing token Humans
  - Oracle text says: `"As long as equipped creature is a Human, it gets an additional +1/+0."`
  - Code does: `state.get_object(creature_id).and_then(|o| registry.card_data(o.card_id)).map(|d| d.subtypes.iter().any(|s| s == "Human")).unwrap_or(false)` (silver_inlaid_dagger.rs lines 16–19). This only checks `registry.card_data()`. For token creatures `card_id = CardId(0)` (sentinel), so `registry.card_data(CardId(0))` returns `None`, and `is_human` is always `false` for tokens. Human tokens store their subtype in `o.subtypes`, not in the registry. The correct pattern (used in dearly_departed.rs lines 52–58) checks both: `registry.card_data(cid).map(|d| d.subtypes.contains("Human")).unwrap_or(false) || obj.subtypes.contains("Human")`. The `update_effects()` in silver_inlaid_dagger.rs omits the `o.subtypes` check, so any Human token equipped with the dagger would receive +2/+0 instead of +3/+0.

### Tricky interactions checked

- "As long as" continuous re-evaluation: FAIL — condition is snapshot at equip time in `update_effects()`; no engine mechanism re-evaluates `instance_continuous_effects` when creature type changes
- Token Human detection: FAIL — `update_effects()` only queries `registry.card_data()`, missing token subtypes stored on `o.subtypes`
- Non-Human creature gets exactly +2/+0: PASS — `update_effects()` sets `instance_continuous_effects = Some([ModifyPT { power: 2, ... }])` for non-Humans, overriding the static `continuous_effects` in `card_data()`; engine's `continuous_pt_mods` uses `instance_continuous_effects` when present
- Human creature (from registry) gets +3/+0 at equip time: PASS — `registry.card_data()` lookup correctly finds "Human" in `subtypes` for real cards like Avacyn's Pilgrim / Champion of the Parish
- Equip only targets own creatures (sorcery speed): PASS — `activated_abilities()` sets `sorcery_speed_only: true` and `target_requirement = Some(CreatureWithFilter(YouControl))`; `is_valid_target()` enforces `o.controller == caster`
- Equipment stays on battlefield when unattached (creature dies): PASS — `on_resolve()` sets `is_equipment = true`; SBA unattaches equipment when attached creature leaves battlefield
- Re-equipping to a different creature updates effects: PASS — `on_activate_ability()` re-calls `update_effects()`, which re-checks `is_human` and writes new `instance_continuous_effects`
- Mana cost: PASS — cast cost `{1}` (GenericMana(1)) and equip cost `{2}` (GenericMana(2)) match oracle
- Card types and subtypes: PASS — `card_types: [Artifact]`, `subtypes: ["Equipment"]` match oracle
- `move_spell_after_resolve` vs `move_object`: PASS for permanents — `on_resolve()` correctly calls `state.move_object(object_id, Zone::Battlefield)` to land the equipment on the battlefield (not `move_spell_after_resolve`, which is correct since this is a permanent)

### Test coverage

- Basic card data (name, cost, types): `tier9_equipment.rs:156` — TESTED
- Non-Human creature gets +2/+0: `tier9_equipment.rs:167` — TESTED
- Human creature gets +3/+0 (registry card): `tier9_equipment.rs:182` — TESTED
- Human token gets +3/+0 (o.subtypes check): NOT TESTED
- "As long as" — bonus drops when equipped Human transforms to non-Human: NOT TESTED
- Equip only targets controller's creatures: `tier9_equipment.rs:99` (Cobbled Wings; pattern shared): TESTED (general equipment test)
- Equipment detaches when creature dies: `tier9_equipment.rs:398` (Cobbled Wings; shared mechanic): TESTED
- Re-equip updates attached_to: `tier9_equipment.rs:421` (Cobbled Wings; shared mechanic): TESTED
