## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +1/+0.
Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"blocks or becomes blocked by a Vampire" — two separate trigger conditions**: Both TriggerKind::Blocks and TriggerKind::BecomesBlocked are declared in triggered_abilities. The trigger dispatch in triggers.rs fires BlocksTrigger (via equipment-attached-to-blocker path, lines 776–798) when the equipped creature is a blocker, and BecomesBlockedTrigger (via equipment-attached-to-attacker path, lines 822–845) when the equipped creature is an attacker. Both paths correctly pass the opposing creature (attacker or blocker respectively) as `other_creature`/`blocker_id` to the on_blocks/on_becomes_blocked handlers. Pass.
- **"destroy that creature. It can't be regenerated"**: The card calls `crate::destruction::try_destroy_no_regen`, which bypasses regeneration shields while still respecting indestructible — matching the oracle text rule that regeneration is prevented but indestructible still protects. Pass.
- **Vampire token subtype check**: Both `registry.card_data(o.card_id)` (for registered cards) and `o.subtypes` (for tokens and objects with instance subtypes) are checked before destroying. A Vampire token created at runtime would have "Vampire" in its `obj.subtypes` and would be caught. Pass.
- **Trigger timing relative to combat damage (ruling: Vampire destroyed before damage)**: The BlockersDeclared event fires during declare_blockers_with_registry; collect_triggers then places the BlocksTrigger/BecomesBlockedTrigger on the stack; these resolve before the CombatDamage step is reached. This matches the ruling. Pass.
- **Equip sorcery-speed only**: `sorcery_speed_only: true` on the ActivatedAbilityDef; the engine enforces this restriction. Pass.
- **Equip only your creatures**: `target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl))` and `is_valid_target` confirms controller matches caster. Pass.
- **+1/+0 continuous effect**: `ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached }` — `effect_applies_to` in state.rs checks `source.attached_to == creature_id`, so the bonus correctly follows the equipped creature and disappears if the equipment is moved or falls off. Pass.
- **Indestructible Vampire not destroyed**: `try_destroy_no_regen` returns `DestroyResult::Indestructible` without moving the object if the Vampire has the Indestructible keyword. Pass.

### Test coverage
- Equipped creature gets +1/+0: `tier9_equipment.rs:313` (`wooden_stake_grants_power`) — TESTED
- Blocks a Vampire → destroy Vampire: `tier9_equipment.rs:329` (`wooden_stake_destroys_vampire_on_block`) — TESTED
- Non-Vampire blocked is not destroyed: `tier9_equipment.rs:365` (`wooden_stake_does_not_destroy_non_vampire`) — TESTED
- Becomes blocked by a Vampire (equipped creature is attacker, Vampire is blocker): NOT TESTED
- "It can't be regenerated" (Vampire with regeneration shield survives try_destroy but not try_destroy_no_regen): NOT TESTED
- Vampire token (subtypes on object, not registry) caught by trigger: NOT TESTED
- Indestructible Vampire survives despite trigger firing: NOT TESTED
- Trigger timing: Vampire destroyed before combat damage dealt: NOT TESTED
- Equip only at sorcery speed: NOT TESTED (but other equipment tests cover this pattern)
