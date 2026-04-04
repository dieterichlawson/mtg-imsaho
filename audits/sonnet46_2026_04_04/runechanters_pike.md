## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **X is constantly updated (ruling 2011-09-22)**: PASS. `dynamic_pt` is called on every `effective_power` invocation via `continuous_pt_mods` → no snapshot; the count is re-computed live each time. Adding or removing instant/sorcery cards from the graveyard is immediately reflected.
- **"your graveyard" scoping**: PASS. `dynamic_pt` is called with the Pike's own `ObjectId` (from `continuous_pt_mods` line 761: `behavior.dynamic_pt(self, source.id)`). It then reads `obj.controller` from the Pike object and filters graveyard cards by `o.owner == controller`. This correctly attributes "your" to the Pike's controller.
- **`EffectScope::Attached` — First Strike only goes to equipped creature**: PASS. `has_keyword` → `has_continuous_effect` → `effect_applies_to(EffectScope::Attached)` checks `get_object(source_id).and_then(|o| o.attached_to).map(|target| target == creature_id)`. Only the creature the Pike is attached to receives First Strike.
- **Equipment stays on battlefield when creature dies**: PASS. `sba.rs` lines 169–188 detect `is_equipment && attached_to.is_some()` and the target has left the battlefield; they set `attached_to = None` without moving the Pike to the graveyard.
- **Equip only at sorcery speed**: PASS. `activated_abilities` returns `sorcery_speed_only: true`; engine enforces this in `legal_actions`.
- **Equip targets only creatures you control**: PASS. `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` in the `ActivatedAbilityDef`, plus `is_valid_target` also checks `o.controller == caster`. Both layers agree.
- **Equip can re-target a different creature**: PASS. `on_activate_ability` simply overwrites `attached_to` with the new target ID; no exclusive lock prevents re-equipping.
- **Only instant and sorcery card types counted (not creature/artifact/etc.)**: PASS. `dynamic_pt` filters `o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery)`. Cards in the graveyard have `card_types` set from `card_data.card_types` during `setup_game`; this field persists across zone changes.
- **Opponent's graveyard cards not counted**: PASS. Filter is `o.owner == controller` where `controller` is the Pike's controller. Opponent-owned cards have a different owner and are excluded.
- **First Strike not in `keywords` vec**: PASS (not a bug). The engine's `Keyword` enum includes `FirstStrike`; it is correctly granted via `ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached }` rather than in `card_data().keywords`, because it is a conditional grant (only while equipped) rather than an intrinsic keyword of the equipment itself.
- **`on_resolve` moves Pike to battlefield (not graveyard)**: PASS. `state.move_object(object_id, Zone::Battlefield)` is used; `move_spell_after_resolve` (graveyard/exile path) is correctly not used for a permanent.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (types, subtype, mana cost): `tier9_cards.rs:runechanters_pike_card_data`
- First strike granted to equipped creature: `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus`
- +X/+0 where X = instant/sorcery count: `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus`
- X dynamically updated as cards enter graveyard (ruling 2011-09-22): `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus` (adds cards mid-test and checks updated power)
- X = 0 when graveyard has no instants/sorceries: `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus` (checks base power before adding GY cards)
- Equip {2} cost and attachment: `tier9_cards.rs:runechanters_pike_equip_ability`
- Equipment detaches when creature dies: `tier9_cards.rs:equipment_detaches_when_creature_dies` (line 582)
- Opponent's graveyard not counted: NOT TESTED
- Equip only targets your own creatures: `tier9_equipment.rs:cobbled_wings_equip_only_your_creatures` (equivalent logic tested for another equipment)
- Re-equip to different creature: `tier9_equipment.rs:equipment_can_be_moved_to_different_creature` (tested for another equipment)
- Equip only at sorcery speed: NOT TESTED explicitly for the Pike
