## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- Fight damage incorrectly doubled by Flail: `mtg-engine/src/combat.rs` lines 452–454
  - Oracle text says: `"If equipped creature would deal combat damage, it deals double that damage instead."` / `"If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead."`
  - Code does: `deal_damage_to_creature` unconditionally applies `amount *= combat_damage_multiplier(state, source, registry); amount *= combat_damage_multiplier(state, target, registry);` for **all** damage routed through it, including non-combat fight damage. The `fight()` function (`combat.rs:158–168`) calls `deal_damage_to_creature` for Prey Upon and Nightfall Predator's fight ability (`prey_upon.rs:48`, `daybreak_ranger.rs:144`). When a creature equipped with Inquisitor's Flail is a participant in a fight, the Flail's doubling fires on that non-combat damage, which violates the oracle text's "combat damage" restriction.

### Tricky interactions checked

- Two Flails on same creature multiply by 4x (ruling 2011-09-22): PASS — `combat_damage_multiplier` uses `1u32 << count` (2^count), so count=2 yields 4. Verified in `combat.rs:311–319` and tested.
- Unblocked equipped creature deals double damage to defending player: PASS — `deal_damage_to_player` applies `combat_damage_multiplier(state, source, registry)` at `combat.rs:507`.
- Blocker deals double combat damage to equipped attacker (second ability): PASS — `deal_damage_to_creature` applies `combat_damage_multiplier(state, target, registry)` at `combat.rs:454`.
- Fight damage (should NOT be doubled per "combat damage" wording): FAIL — `fight()` routes through `deal_damage_to_creature` which applies the Flail multiplier unconditionally.
- Flail detaches via SBA when equipped creature leaves battlefield: PASS — `sba.rs:169–188` has a dedicated `detach_equipment` pass that clears `attached_to` on equipment whose target is no longer on the battlefield.
- Equip ability only targets creatures you control: PASS — `activated_abilities` uses `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)`; `matches_ability_target_filter` checks `obj.controller == controller` (`engine.rs:1242`).
- "Another creature" restriction (equipped creature should not apply second ability to its own self-damage): effectively N/A in practice — a creature cannot deal combat damage to itself in normal MTG, so no erroneous double-application occurs.
- Trample with Flail (ruling: divide original amounts, then double each): PASS — `deal_damage_step` computes `remaining_power` from the undoubled effective power, assigns minimum-lethal to blocker and rest to player, then each call to `deal_damage_to_creature` / `deal_damage_to_player` applies the multiplier independently, matching the ruling example.
- Flail unequipped (unattached on battlefield) applies no effect: PASS — `effect_applies_to` for `EffectScope::Attached` checks `source.attached_to == Some(creature_id)`; an unattached Flail has `attached_to = None` and contributes count 0.
- `on_resolve` uses `move_object` to Battlefield (correct for permanents, not `move_spell_after_resolve`): PASS.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Equipped creature deals double damage to defending player: `tests/inquisitors_flail.rs:21` (`doubles_damage_to_player`)
- Equipped creature deals double damage to blocking creature: `tests/inquisitors_flail.rs:44` (`doubles_damage_to_creature`)
- Equipped creature takes double damage from blocker: `tests/inquisitors_flail.rs:67` (`doubles_damage_taken_from_blocker`)
- No doubling when Flail is not attached: `tests/inquisitors_flail.rs:90` (`no_doubling_without_flail`)
- Two Flails = 4x damage (ruling 2011-09-22): `tests/inquisitors_flail.rs:113` (`two_flails_quadruple_damage`)
- Fight damage should NOT be doubled (Prey Upon / Nightfall Predator): NOT TESTED
- Trample + Flail (divide original amounts, then double): NOT TESTED
- Flail detaches when equipped creature dies: NOT TESTED
