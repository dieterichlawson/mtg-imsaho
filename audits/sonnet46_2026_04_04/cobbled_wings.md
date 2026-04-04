## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying grant via `EffectScope::Attached` continuous re-evaluation: PASS — `effect_applies_to` reads `source.attached_to` dynamically on every `has_keyword` call (`state.rs:700-705`), so the grant is always live for the currently-attached creature and not a snapshot.
- Re-equipping to a different creature (old creature loses flying): PASS — `on_activate_ability` simply overwrites `obj.attached_to = Some(*creature_id)`; because the effect scope is evaluated live, the old creature immediately loses flying and the new one gains it. Confirmed by test `equipment_can_be_moved_to_different_creature`.
- Equip only at sorcery speed: PASS — `sorcery_speed_only: true` is correctly gated by `is_sorcery_speed = state.step.is_main_phase() && state.stack.is_empty() && state.active_player == player` (`engine.rs:301-304, 360`).
- Equip only to creatures you control: PASS — `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` in target generation (`engine.rs:1305-1312`) plus the `is_valid_target` check (`cobbled_wings.rs:52-53`) both enforce controller == caster.
- Equip cannot target opponent's creatures: PASS — `TargetFilter::YouControl` filters out non-controller objects in `matches_ability_target_filter` (`engine.rs:1242`). Confirmed by test `cobbled_wings_equip_only_your_creatures`.
- Flying not applied when unattached (Wings just entered battlefield): PASS — `EffectScope::Attached` returns false when `attached_to` is `None`.
- Equipment stays on battlefield when equipped creature dies (SBA detach, not destroy): PASS — SBA (`sba.rs:168-188`) detects `is_equipment && attached_to.is_some()` pointing to a non-battlefield object and sets `attached_to = None`, explicitly excluded from the aura-goes-to-graveyard rule (`sba.rs:157`). Confirmed by test `equipment_detaches_when_creature_dies`.
- `on_resolve` uses `move_object(Zone::Battlefield)` not `move_spell_after_resolve`: PASS — This is correct for a permanent resolving onto the battlefield; `move_spell_after_resolve` is for spells that go to graveyard/exile after resolving.
- Equip can be activated multiple times per turn: PASS — `once_per_turn: false`, so each activation pays {1} and updates `attached_to` independently.
- Equipment does not grant Flying to creatures in graveyard after death (before SBA): PASS — `has_keyword` (`state.rs:988-990`) immediately returns false if the object is not in `Zone::Battlefield`.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (name, cost {2}, Artifact type, Equipment subtype, no P/T): `tier9_equipment.rs:54-64` — TESTED
- Enters battlefield with `is_equipment = true` and `attached_to = None`: `tier9_equipment.rs:67-77` — TESTED
- Equipped creature gets Flying: `tier9_equipment.rs:80-96` — TESTED
- Equip only targets your own creatures: `tier9_equipment.rs:99-116` — TESTED
- Equipment detaches (stays on battlefield) when creature dies: `tier9_equipment.rs:397-418` — TESTED
- Re-equipping to a second creature: `tier9_equipment.rs:421-440` — TESTED
- Full cast-then-equip flow: `tier9_equipment.rs:443-463` — TESTED
- Equip sorcery-speed-only restriction: NOT TESTED (no test verifying equip is unavailable at instant speed or with non-empty stack)
- Flying not applied while unattached: implicitly covered by `cobbled_wings_enters_as_equipment` and `cobbled_wings_cast_and_equip_full_flow`, NOT TESTED explicitly
