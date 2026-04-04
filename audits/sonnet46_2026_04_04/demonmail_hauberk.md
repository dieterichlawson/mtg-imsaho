## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +4/+2.
Equip—Sacrifice a creature.
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- Player cannot choose which creature to sacrifice for the Equip cost
  - Oracle text says: `"Equip—Sacrifice a creature."` (the player pays the cost by sacrificing a creature of their choice)
  - Code does: In `mtg-engine/src/engine.rs` lines 1750–1759, `SacrificeCost::SacrificeCreature` is handled by auto-selecting the first eligible creature from `objects_in_zone(...).iter().find(|o| o.power.is_some())`. Since `objects_in_zone` iterates a `HashMap`, the "first" creature is non-deterministic. The player is never asked which creature to sacrifice.
  - Additionally, in `legal_actions` (engine.rs lines 368–373), the engine only checks that at least one creature exists (`any(|o| o.power.is_some())`), then generates one `ActivateAbility` action per equip target — it does not generate separate actions for each possible (sacrifice-target, equip-target) pair. The `Action::ActivateAbility` struct has no field for the chosen sacrifice, so the choice is architecturally absent.
  - The Scryfall ruling states: "You can sacrifice the creature Demonmail Hauberk is equipping in order to equip it to another creature." This explicitly confirms the player has a choice. With auto-selection, the engine may sacrifice the wrong creature (e.g., the currently equipped one when the player wants to sacrifice a different one, or vice versa).

### Tricky interactions checked

- **Player choice of sacrifice creature**: FAIL — engine auto-selects first eligible creature non-deterministically; player cannot choose (see issue above)
- **Sacrificing the currently equipped creature to re-equip**: FAIL — the ruling specifically calls this out as a valid player strategy, but it cannot be reliably executed because auto-selection may pick a different creature
- **Equip ability availability — only on battlefield, not as creature**: PASS — ability is gated on `obj.zone == Zone::Battlefield && obj.power.is_none()` (line 59 of card file), correctly preventing the ability on creatures or from graveyard/hand
- **Equip ability is sorcery-speed only**: PASS — `sorcery_speed_only: true` (line 68 of card file)
- **Equip can only target your own creatures**: PASS — `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` plus `is_valid_target` checking `o.controller == caster`
- **+4/+2 bonus applies continuously to attached creature**: PASS — `ContinuousEffect::ModifyPT { power: 4, toughness: 2, scope: EffectScope::Attached }` is evaluated dynamically via `continuous_pt_mods` every call to `effective_power`/`effective_toughness`
- **Equipment stays on battlefield when creature dies**: PASS — SBAs in `sba.rs` lines 168–188 detach equipment (set `attached_to = None`) when attached creature leaves battlefield, and skip the aura-destruction SBA for `is_equipment = true`
- **Equipment enters unattached**: PASS — `on_resolve` moves to battlefield but does not set `attached_to`; confirmed by `equipment_enters_unattached` test
- **Re-equipping to a different creature**: PASS — `on_activate_ability` simply overwrites `attached_to` with the new target; old creature loses the bonus immediately (continuous effect re-evaluates)
- **Spell resolution (permanent goes to battlefield, not graveyard)**: PASS — `on_resolve` calls `state.move_object(object_id, Zone::Battlefield)`; `resolve_spell` in `stack.rs` lines 107–111 only calls `move_spell_after_resolve` if still in `Zone::Stack`, which it is not
- **Mana cost {4}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(4)])`, mana_value = 4
- **Card types and subtypes**: PASS — `card_types: vec![CardType::Artifact]`, `subtypes: vec!["Equipment".into()]`

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Card data (cost, type, subtype): `tier9_cards.rs:83` (`demonmail_hauberk_card_data`) — TESTED
- Equipment enters battlefield unattached: `tier9_cards.rs:561` (`equipment_enters_unattached`) — TESTED
- Equip sacrifices a creature and attaches: `tier9_cards.rs:93` (`demonmail_hauberk_equip_sacrifices_creature`) — TESTED (but accepts either creature being sacrificed, does not verify player choice)
- Player chooses which creature to sacrifice: NOT TESTED
- Ruling — sacrificing the equipped creature to move to another: NOT TESTED
- +4/+2 bonus on attached creature: `tier9_cards.rs:132–135` (inline in `demonmail_hauberk_equip_sacrifices_creature`, conditional on `b_zone == Zone::Battlefield`) — TESTED (weakly, only if the auto-sacrifice picks creature_a)
- Equipment detaches when creature dies: `tier9_equipment.rs:398` (`equipment_detaches_when_creature_dies`, uses Cobbled Wings) — TESTED (general equipment mechanic)
- Re-equipping to different creature: `tier9_equipment.rs:421` (`equipment_can_be_moved_to_different_creature`, uses Cobbled Wings) — TESTED (general equipment mechanic)
- Equip only targets your own creatures: `tier9_equipment.rs:99` (`cobbled_wings_equip_only_your_creatures`) — TESTED (general equipment mechanic)
- Equip is sorcery-speed: NOT TESTED explicitly for Demonmail Hauberk
