---
id: reaper_from_the_abyss-01
status: new
card: Reaper from the Abyss
audit_run_id: 2026-04-19-reaper_from_the_abyss-audit
audit_model: sonnet
audit_tokens: 25098
audit_duration: 451
---

## Audit Finding

**Oracle text:**
> At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.

**Code:**
> PendingTrigger::EndStepTrigger { object_id, card_id, chosen_targets, .. } => {
    if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
        if let Some(behavior) = registry.get(card_id) {
            behavior.on_end_step(state, object_id, &chosen_targets, registry);
        }
    }
}

// and in reaper_from_the_abyss.rs:
fn on_end_step(&self, ...) {
    // Must still be on the battlefield.
    if !state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) {
        return;
    }

**Description:**
Per CR 112.7a, a triggered ability exists on the stack independently of its source — once the Reaper's morbid end-step trigger is on the stack, destroying the Reaper in response must not counter the trigger. The dispatch arm for `PendingTrigger::EndStepTrigger` in `resolve_next_trigger` (triggers.rs:1391) gates the call to `on_end_step` behind `state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)`. If an opponent destroys the Reaper after the trigger is on the stack but before it resolves, this check silently drops the trigger and the target creature survives, which is rules-wrong. The `UpkeepTrigger` arm in the same match (lines 1385–1388) has no such zone guard, and the ETB trigger arm (line 1326) explicitly comments 'Per MTG rules, ETB triggers resolve even if the source has left the battlefield' — making the EndStep omission inconsistent. The Reaper's own `on_end_step` handler (reaper_from_the_abyss.rs:65) adds a redundant zone check that would equally suppress the effect if the engine-level guard were removed first; the destroy target is a different creature and the Reaper's zone is irrelevant to whether that destruction can proceed.

**Engine path:** mtg-engine/src/triggers.rs:1391

**Required check:** 8b

**Affected cards:**
- Mayor of Avabruck
- Cloistered Youth
- Civilized Scholar

## Tests

### end_step_trigger_resolves_after_reaper_destroyed
Scenario: Reaper's morbid end-step trigger is on the stack targeting an opponent's creature; opponent destroys the Reaper in response; trigger should still resolve and destroy the target creature.

