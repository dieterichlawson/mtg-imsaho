---
id: daybreak_ranger-03
status: fixed
card: Daybreak Ranger
audit_run_id: 2026-04-19-daybreak_ranger-audit
audit_model: sonnet
audit_tokens: 29964
audit_duration: 567
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: combat::fight now routes both hits through damage::deal_damage, which applies PreventDamageRemoveCounter; regression coverage present
---

## Audit Finding

**Oracle text:**
> {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)

**Code:**
> fn deal_fight_damage(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    registry: &CardRegistry,
) {
    // Protection: if target has protection from the source, prevent damage.
    if has_protection_from_creature(state, target, source, registry) {
        return;
    }
    ...
    if let Some(obj) = state.get_object_mut(target) {
        obj.damage_marked += amount;

**Description:**
`deal_fight_damage` in combat.rs (line 186) directly writes `obj.damage_marked += amount` instead of routing through `apply_pending_effect(PendingEffect::DealDamage)`. The central handler at engine.rs:3434 first checks for `PreventDamageRemoveCounter` continuous effects (Unbreathing Horde's damage-prevention-by-counter-removal, CR 614.1a): if the target has this effect and a +1/+1 counter, damage is prevented and a counter is removed instead. `deal_fight_damage` skips this check entirely. When Nightfall Predator fights an Unbreathing Horde with a +1/+1 counter, the damage is applied as normal damage marks instead of being prevented and consuming the counter — incorrect per CR 614.1a. Note that `deal_fight_damage` does correctly handle protection-from-source (via `has_protection_from_creature`) and lifelink; the sole gap relative to the central handler is the `PreventDamageRemoveCounter` replacement effect.

**Engine path:** mtg-engine/src/combat.rs:186

**Required check:** 8e

**Affected cards:**
- Nightfall Predator
- Prey Upon

## Tests

### nightfall_predator_fight_damage_prevented_by_unbreathing_horde_counter
Scenario: Place Nightfall Predator (back face, 4/4) and an Unbreathing Horde with one +1/+1 counter on the battlefield (opponents). Activate Nightfall Predator's {R},{T} fight ability targeting Unbreathing Horde. After resolution, assert Unbreathing Horde's damage_marked is 0 and its +1/+1 counter count decreased by 1 (damage was prevented per CR 614.1a).

