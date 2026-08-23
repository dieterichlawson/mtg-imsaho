---
id: blazing_torch-01
status: fixed
card: Blazing Torch
audit_run_id: 2026-04-19-blazing_torch-audit
audit_model: sonnet
audit_tokens: 23515
audit_duration: 1971
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: inline damage replaced by damage::deal_damage (blazing_torch.rs:126); regression coverage present
---

## Audit Finding

**Oracle text:**
> Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."

**Code:**
> if let Some(obj) = state.get_object_mut(*target_id) {
    obj.damage_marked += 2;
    obj.damaged_by.push(damage_source);
}

**Description:**
The damage ability (ability_index == 1) applies damage to object targets by writing obj.damage_marked += 2 directly (blazing_torch.rs:125) instead of routing through the central apply_pending_effect(PendingEffect::DealDamage) handler in engine.rs. The central handler (engine.rs:3457) checks has_protection_from before marking damage, preventing it when protection applies; the inline path skips this check entirely. The second ruling confirms prevention must apply: "It could target a creature with protection from artifacts, but all the damage would be prevented." Additionally, the central handler (engine.rs:3465-3476) removes loyalty counters when the target is a planeswalker rather than incrementing damage_marked; the inline path unconditionally increments damage_marked for all object targets, so a token planeswalker targeted by the torch would have damage_marked set instead of losing loyalty. The inline path also skips the PreventDamageRemoveCounter replacement effect (Unbreathing Horde pattern, engine.rs:3434-3456). Player targets (blazing_torch.rs:135-137) are functionally equivalent to the central handler in the current engine, so the critical divergence is for object targets.

**Engine path:** mtg-engine/src/cards/isd/blazing_torch.rs:124

**Required check:** 8e

## Tests

### damage_prevented_by_protection_from_artifacts
Scenario: Blazing Torch is equipped to a creature; the controller activates the damage ability targeting a creature with protection from artifacts; the 2 damage should be prevented and the target creature's damage_marked should remain 0.

### damage_to_planeswalker_removes_loyalty
Scenario: Blazing Torch is equipped; the controller activates the damage ability targeting a token planeswalker; the planeswalker should lose 2 loyalty counters, not have damage_marked set.

