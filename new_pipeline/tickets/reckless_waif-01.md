---
id: reckless_waif-01
status: new
card: Reckless Waif
audit_run_id: 2026-04-19-reckless_waif-audit
audit_model: sonnet
audit_tokens: 13940
audit_duration: 280
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

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

**Description:**
Both faces carry intervening-if triggers (CR 603.4): front face fires only 'if no spells were cast last turn'; back face fires only 'if a player cast two or more spells last turn'. Per CR 603.4, an intervening-if condition must be true at the moment the trigger event occurs for the trigger to be placed on the stack at all. The dispatch in triggers.rs (StepStarted handler) calls `face_trigger_description` to detect that the card has an upkeep trigger, then unconditionally queues it — the only guard is the `step_trigger_scope` player-scoping check. There is no call to `should_transform` or any equivalent hook before the `PendingTrigger::UpkeepTrigger` is pushed. As a result, the transform trigger appears on the stack on every single upkeep regardless of the spell-count condition, giving both players a spurious priority window to respond to a trigger that will silently do nothing. The condition is only checked at resolution inside `on_upkeep` (reckless_waif.rs:90), which is too late per CR 603.4.

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 8b

**Affected cards:**
- Village Ironsmith
- Gatstaf Shepherd
- Tormented Pariah
- Kruin Outlaw
- Daybreak Ranger
- Instigator Gang
- Mayor of Avabruck
- Villagers of Estwald
- Hanweir Watchkeep
- Grizzled Outcasts
- Ulvenwald Mystics

## Tests

### front_face_trigger_suppressed_when_spell_was_cast
Scenario: Reckless Waif is on the battlefield (front face); a spell was cast last turn; at the beginning of the upkeep, no transform trigger should appear on the stack.

### back_face_trigger_suppressed_when_fewer_than_two_spells_cast
Scenario: Merciless Predator (back face) is on the battlefield; only one spell was cast last turn; at the beginning of the upkeep, no transform-back trigger should appear on the stack.

