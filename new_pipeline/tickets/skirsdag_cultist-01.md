---
id: skirsdag_cultist-01
status: fixed
card: Skirsdag Cultist
audit_run_id: 2026-04-19-skirsdag_cultist-audit
audit_model: sonnet
audit_tokens: 29132
audit_duration: 1795
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: inline damage replaced by damage::deal_damage (skirsdag_cultist.rs:56); regression coverage present
---

## Audit Finding

**Oracle text:**
> This creature deals 2 damage to any target.

**Code:**
> if let Some(obj) = state.get_object_mut(*target_id) {
    if obj.zone == Zone::Battlefield {
        obj.damage_marked += 2;
        obj.damaged_by.push(object_id);
    }
}

**Description:**
The on_activate_ability handler (skirsdag_cultist.rs:54-58) directly writes obj.damage_marked += 2 for Object targets instead of routing through apply_pending_effect(PendingEffect::DealDamage). The central handler (engine.rs:3432-3500) performs four checks the inline path omits: (1) protection from source — has_protection_from(target_id, source_id) prevents damage and suppresses the damage event (engine.rs:3457-3461); (2) PreventDamageRemoveCounter replacement — damage to Unbreathing Horde-style targets is prevented and a +1/+1 counter removed instead (engine.rs:3434-3455); (3) planeswalker loyalty removal — damage to a planeswalker removes loyalty counters rather than incrementing damage_marked, which has no SBA consequence for planeswalkers (engine.rs:3465-3475); (4) lifelink — if Skirsdag Cultist has been granted lifelink, the controller gains life equal to damage dealt (engine.rs:3486-3498). Additionally, the inline code emits NonCombatDamageDealt unconditionally, so damage-watch triggers fire even when protection should have prevented both the damage and its event.

**Engine path:** mtg-engine/src/cards/isd/skirsdag_cultist.rs:54

**Required check:** 8e

**Affected cards:**
- Blazing Torch
- Corpse Lunge
- Ashmouth Hound

## Tests

### skirsdag_cultist_damage_ignores_protection_from_red
Scenario: A creature with protection from Red is on the battlefield; Skirsdag Cultist activates its ability targeting that creature; the damage should be prevented but the inline path applies it and emits the damage event.

### skirsdag_cultist_damage_to_planeswalker_marks_damage_not_loyalty
Scenario: A planeswalker token (card_types contains Planeswalker) is targeted; after Skirsdag Cultist's ability resolves, 2 loyalty counters should be removed from the planeswalker, but the inline path increments damage_marked instead, leaving loyalty unchanged.

### skirsdag_cultist_lifelink_grants_life_on_activation
Scenario: Skirsdag Cultist has been granted lifelink by a temporary effect; when it deals 2 damage via its activated ability, its controller should gain 2 life, but the inline path has no lifelink check.

