---
id: hanweir_watchkeep-01
status: new
card: Hanweir Watchkeep
audit_run_id: 2026-04-19-hanweir_watchkeep-audit
audit_model: sonnet
audit_tokens: 22396
audit_duration: 728
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature. (front face) / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature. (back face)

**Code:**
> let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
if !desc.is_empty() {
    if behavior.step_trigger_scope(&kind, is_transformed) == crate::cards::TriggerScope::Your
        && controller != active_player
    {
        continue;
    }
    let trigger = match kind {
        crate::cards::TriggerKind::Upkeep => PendingTrigger::UpkeepTrigger {
            object_id: obj_id,
            card_id,
            controller,
            description: desc,
            chosen_targets: Vec::new(),
        },
        // ...
    };
    ap_triggers.push(trigger);
}

**Description:**
Both upkeep triggers on Hanweir Watchkeep and Bane of Hanweir are intervening-if triggers: the oracle text reads 'At the beginning of each upkeep, IF [condition], transform.' Per CR 603.4, a triggered ability with an intervening 'if' clause may only be placed on the stack when the condition is true at the time the trigger event occurs AND when the ability would resolve. The engine dispatch in triggers.rs (the StepStarted/Upkeep handler) queues an UpkeepTrigger for every permanent whose face declares a non-empty Upkeep trigger description, with no check of the intervening-if condition. The condition is only evaluated at resolution time inside on_upkeep(), which correctly calls should_transform() before applying the transformation. The result: on any upkeep where a spell was cast last turn (front face condition is false), or where fewer than two spells were cast last turn (back face condition is false), the trigger still appears on the stack and players receive a priority window to respond to a trigger that should never have been placed on the stack. This violates CR 603.4.

**Engine path:** mtg-engine/src/triggers.rs:883

**Required check:** 8b

**Affected cards:**
- Village Ironsmith
- Kruin Outlaw

## Tests

### front_face_trigger_not_queued_when_spell_cast_last_turn
Scenario: Hanweir Watchkeep is on the battlefield (front face); a spell was cast last turn; at the start of the upkeep, no trigger should appear on the stack and priority should pass directly, not pause for a trigger.

### back_face_trigger_not_queued_when_fewer_than_two_spells_cast
Scenario: Bane of Hanweir is on the battlefield (back face, transformed); exactly one spell was cast last turn; at the start of the upkeep, no trigger should appear on the stack and priority should pass directly.

