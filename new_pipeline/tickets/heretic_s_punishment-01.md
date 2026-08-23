---
id: heretic_s_punishment-01
status: new
card: Heretic's Punishment
audit_run_id: 2026-04-19-heretic_s_punishment-audit
audit_model: sonnet
audit_tokens: 23651
audit_duration: 422
---

## Audit Finding

**Oracle text:**
> This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.

**Code:**
> Target::Object(target_id) => {
    if let Some(obj) = state.get_object_mut(*target_id) {
        if obj.zone == Zone::Battlefield {
            obj.damage_marked += max_mv;
            obj.damaged_by.push(object_id);
        }
    }
    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
        source: object_id,
        target: crate::events::DamageTarget::Object(*target_id),
        amount: max_mv,
    });
}

**Description:**
The ability inlines permanent-target damage by directly writing `obj.damage_marked += max_mv` (heretics_punishment.rs:109) instead of routing through the central `apply_pending_effect` / `PendingEffect::DealDamage` path. The central path (engine.rs:3432–3499) performs four checks that the inline path skips: (a) the `has_protection_from` guard at engine.rs:3457, so a creature with protection from red is damaged even though protection prevents all damage from that source (CR 702.16b); (b) the planeswalker loyalty-removal branch at engine.rs:3469–3475, so a targeted planeswalker accumulates raw `damage_marked` on the object instead of having loyalty counters removed (CR 120.3c); (c) the `PreventDamageRemoveCounter` replacement-effect check at engine.rs:3434–3455; and (d) the lifelink controller-gain at engine.rs:3486–3498. Note: the player-target branch (heretics_punishment.rs:119–133) is functionally equivalent to the central player-damage path (engine.rs:3502–3513), so the player branch is not a separate bug.

**Engine path:** mtg-engine/src/cards/isd/heretics_punishment.rs:107

**Required check:** 8e

## Tests

### protection_from_red_prevents_damage
Scenario: Heretic's Punishment mills three cards with positive mana values and targets a creature with protection from red; the damage should be prevented entirely (0 damage reaches the creature).

### planeswalker_target_removes_loyalty_counters
Scenario: Heretic's Punishment mills three cards with MV ≥ 1 and targets a planeswalker; the ability should remove loyalty counters equal to the highest MV rather than setting damage_marked on the object.

