---
id: thraben_sentry-01
status: new
card: Thraben Sentry
audit_run_id: 2026-04-19-thraben_sentry-audit
audit_model: sonnet
audit_tokens: 16871
audit_duration: 354
---

## Audit Finding

**Oracle text:**
> Whenever another creature you control dies, you may transform this creature.

**Code:**
> let has_death_trigger = registry.get(watcher_card_id)
    .is_some_and(|b| b.card_data().triggered_abilities.iter()
        .any(|t| t.kind == crate::cards::TriggerKind::AnyCreatureDies));

**Description:**
When Thraben Sentry has transformed to Thraben Militia (back face), the death-watch dispatch at triggers.rs:662-664 checks `b.card_data().triggered_abilities`, which always returns front-face data. Since the front face (Thraben Sentry) has an `AnyCreatureDies` triggered ability, a `DeathWatch` trigger is created and pushed onto the stack even though the active face is Thraben Militia, which has no triggered abilities. Per CR 712.8d, a DFC on the battlefield has only the characteristics of its current face-up face; Thraben Militia has empty `triggered_abilities`, so no trigger should be created. The `on_any_creature_dies` resolution handler does suppress the actual transform via its `is_transformed` guard, but the spurious trigger is already on the stack — an observable game state that grants both players an illegitimate priority window. The fix is to include `o.is_transformed` in the watchers collection at triggers.rs:654-658, then at line 662-664 branch on `is_transformed` to check `b.back_face_data().triggered_abilities` instead of `b.card_data().triggered_abilities` when true.

**Engine path:** mtg-engine/src/triggers.rs:662

**Required check:** 8b

**Affected cards:**
- Thraben Sentry

## Tests

### no_trigger_when_transformed
Scenario: Sentry has already transformed to Thraben Militia; another creature the same player controls dies; verify no DeathWatch trigger is added to the stack

