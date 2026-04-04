## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.)
Equip {3}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- **Duplicate equip action generated via attached-aura loop enables broken re-equip** (`mtg-engine/src/engine.rs:331-338`, `mtg-engine/src/cards/isd/mask_of_avacyn.rs:59-65`)
  - Oracle text says: `Equip {3}` — paying {3} at sorcery speed attaches the Mask to a creature you control.
  - Code does: When the Mask is already attached to a creature, `legal_actions` iterates the equipped creature as `obj_id` and the attached-aura loop (engine.rs:331-338) calls `MaskOfAvacyn::activated_abilities(state, creature.id, registry)`. The zone check `state.get_object(object_id).map(|o| o.zone == Zone::Battlefield)` passes (the creature is on the battlefield), so the equip ability is returned and a duplicate `Action::ActivateAbility { object_id: creature.id, ability_index: 0, targets: [...] }` is added to legal actions alongside the correct `Action::ActivateAbility { object_id: mask.id, ... }`. When this incorrect action is executed, the engine calls `MaskOfAvacyn::on_activate_ability(&mut state, creature.id, ...)` (mask_of_avacyn.rs:59-65), which does `state.get_object_mut(creature.id)` and sets `creature.attached_to = Some(target_id)` — updating the *creature's* `attached_to` field instead of the Mask's. The Mask's `attached_to` field remains pointing to the old creature; the continuous effects (`EffectScope::Attached`) still resolve against `mask.attached_to`, so the +1/+2 and hexproof do not transfer to the new target. The re-equip silently fails.

### Tricky interactions checked

- **+1/+2 continuous effect via `EffectScope::Attached`**: The `effect_applies_to` function in `state.rs:700-705` checks `source.attached_to == Some(creature_id)`, which correctly applies the modifier only to the currently equipped creature. Pass.
- **Hexproof continuous grant via `EffectScope::Attached`**: `has_keyword` → `has_continuous_effect` → `effect_applies_to` correctly resolves `GrantKeyword { keyword: Keyword::Hexproof, scope: EffectScope::Attached }` for the equipped creature. Pass.
- **Hexproof prevents opponent targeting of equipped creature**: `can_be_targeted` (engine.rs:758-767) checks `state.has_keyword(target_id, Keyword::Hexproof, registry)` and returns false if `controller != caster`. Opponents are blocked from targeting the equipped creature. Pass.
- **Ruling — Mask on opponent's creature, Mask-controller can't target it**: `can_be_targeted` compares the creature's `controller` (opponent) against `caster` (Mask controller); `controller != caster` → returns false. Mask controller cannot target the opponent-controlled hexproof creature. Pass.
- **Equip only targets creatures you control**: `TargetFilter::YouControl` in the ability definition (mask_of_avacyn.rs:41) + `is_valid_target` (mask_of_avacyn.rs:50-57) both enforce `o.controller == caster`. Pass.
- **Equip is sorcery speed only**: `sorcery_speed_only: true` in the `ActivatedAbilityDef` (mask_of_avacyn.rs:43), enforced in engine.rs:360. Pass.
- **Equipment detaches when equipped creature dies**: SBA (sba.rs:168-188) scans for equipment whose `attached_to` points to a creature no longer on the battlefield, and clears `attached_to`. Pass.
- **Initial equip (Mask unattached) generates correct single action**: When the Mask is not yet attached, only the Mask's own iteration (`obj_id = mask.id`) generates the equip action. `on_activate_ability` is called with the Mask's ID and correctly sets `mask.attached_to = Some(creature_id)`. Pass.
- **Re-equip (Mask already attached) generates duplicate action**: When the Mask is attached, BOTH the correct action (`object_id: mask.id`) and an incorrect action (`object_id: creature.id`) appear in legal actions. See Code issues above. **FAIL.**
- **Player can equip to own hexproof creature**: `can_be_targeted` only blocks targeting when `controller != caster`. The player equipping their own hexproof creature has `controller == caster`, so the target is valid. Pass.
- **Hexproof does not retroactively fizzle resolved spells**: `is_target_legal` in `stack.rs:8-41` only checks zone at resolution time, not hexproof. Per MTG rules, hexproof only applies when choosing targets. Pass.
- **Mask subtypes include "Equipment"**: card_data correctly lists `subtypes: vec!["Equipment".into()]`. Pass.
- **`keywords: vec![]` omits Equip**: Scryfall marks "Equip" as a keyword, but the engine's `Keyword` enum does not include Equip; it is implemented as an activated ability. Not an issue per audit rules.
- **Oracle text field matches Scryfall (minus parenthetical)**: Parenthetical reminder text `(It can't be...)` is omitted consistently with all other hexproof cards in the engine (Geist of Saint Traft, Lumberknot). Not an issue.

### Test coverage

- **+1/+2 applied to equipped creature**: `tier9_equipment.rs:134` (`mask_of_avacyn_grants_pt_and_hexproof`) — TESTED
- **Hexproof granted to equipped creature**: `tier9_equipment.rs:134` (`mask_of_avacyn_grants_pt_and_hexproof`) — TESTED
- **Equip only targets creatures you control**: `tier9_equipment.rs:99` (`cobbled_wings_equip_only_your_creatures`, same mechanic) — TESTED for Cobbled Wings, NOT TESTED for Mask of Avacyn specifically
- **Equipment detaches when creature dies**: `tier9_equipment.rs:398` (`equipment_detaches_when_creature_dies`) — TESTED for Cobbled Wings, NOT TESTED for Mask of Avacyn
- **Re-equip Mask to different creature**: NOT TESTED (only tested for Cobbled Wings via `equipment_can_be_moved_to_different_creature`, but that test uses explicit `object_id == equipment_id` filtering in the `equip` helper, sidestepping the duplicate action bug)
- **Duplicate equip action from attached-aura loop**: NOT TESTED
- **Hexproof prevents opponent targeting of equipped creature**: NOT TESTED
- **Ruling — opponent-controlled equipped creature targeting**: NOT TESTED
- **Equip is sorcery speed only**: NOT TESTED for Mask of Avacyn specifically
