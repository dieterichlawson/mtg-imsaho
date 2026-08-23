---
id: evil_twin-03
status: new
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
---

## Audit Finding

**Oracle text:**
> You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."

**Code:**
> let entering_copy = state.get_object(id).is_some_and(|o| o.entering_copy_source)
    || state.get_object(id)
        .and_then(|o| registry.get(o.card_id))
        .is_some_and(super::cards::CardBehavior::enters_as_copy);
match effective_t {
    Some(t) if t <= 0 && !entering_copy => {
        zero_toughness_ids.push(id);
    }
    Some(t) if !entering_copy && (i32::try_from(damage).unwrap_or(i32::MAX) >= t || (deathtouch && damage > 0)) => {
        destroyed_ids.push(id);
    }
    _ => {}
}

**Description:**
`entering_copy_source` is set to `true` in `on_enter_battlefield` to protect the 0/0 Evil Twin from SBA while the copy choice is pending. When the player declines to copy, the flag is explicitly cleared (evil_twin.rs:62). But when the copy succeeds — when `CopyCreature` resolves (engine.rs:3761-3795) — the flag is never cleared. After a successful copy, Evil Twin's `card_id` changes to the copied creature's; `registry.get(new_card_id).enters_as_copy()` returns `false`, so the behavior-based SBA guard (second OR branch, sba.rs:72-73) is false. But `entering_copy_source` stays `true` permanently, keeping the first OR branch true. This makes `entering_copy = true` for the rest of Evil Twin's existence on the battlefield, and both the lethal-damage SBA (704.5g) and the 0-toughness SBA (704.5f) are skipped. Evil Twin becomes invulnerable to SBA — it will not die from lethal damage in combat or from toughness being reduced to 0.

**Engine path:** mtg-engine/src/engine.rs:3761

**Required check:** 8a

## Tests

### evil_twin_dies_to_lethal_damage_after_copy
Scenario: Evil Twin copies a 2/2 creature and is dealt 3 damage; SBA should destroy Evil Twin (damage >= toughness) but currently skips the check because entering_copy_source is permanently true.

### evil_twin_dies_to_zero_toughness_after_copy
Scenario: Evil Twin copies a 2/2 creature and an effect reduces its toughness to 0; SBA should destroy Evil Twin (toughness <= 0) but currently skips the check.

