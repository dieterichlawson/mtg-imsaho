---
id: tormented_pariah-01
status: new
card: Tormented Pariah
audit_run_id: 2026-04-19-tormented_pariah-audit
audit_model: sonnet
audit_tokens: 12840
audit_duration: 468
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature. [back face] At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Code:**
> let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
if !desc.is_empty() {
    if behavior.step_trigger_scope(&kind, is_transformed) == crate::cards::TriggerScope::Your
        && controller != active_player
    {
        continue;
    }
    let trigger = match kind {
        crate::cards::TriggerKind::Upkeep => PendingTrigger::UpkeepTrigger { ... }

**Description:**
The `StepStarted` handler in `triggers.rs` (around line 843) queues upkeep triggers for every battlefield permanent whose current face declares an upkeep trigger — it never evaluates the intervening-if condition before queueing. Per CR 603.4, a triggered ability phrased 'At the beginning of each upkeep, if [condition], [effect]' must evaluate the condition when the trigger event occurs; the trigger is only placed on the stack if the condition is true at that moment. Both faces of Tormented Pariah have intervening-if clauses: the front face requires 'no spells were cast last turn' and the back face (Rampaging Werewolf) requires 'a player cast two or more spells last turn'. When exactly one spell was cast last turn, neither condition is satisfied, yet the engine still places a trigger on the stack, granting players a spurious priority window to respond to a trigger that will do nothing on resolution. The card's `should_transform` method correctly implements both conditions but is only called inside `on_upkeep` (at resolution), not at trigger-creation time. The fix requires the `StepStarted` dispatch loop to call an equivalent of `should_transform` before creating each `UpkeepTrigger`.

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 8b

**Affected cards:**
- Reckless Waif
- Gatstaf Shepherd
- Village Ironsmith
- Mayor of Avabruck
- Daybreak Ranger
- Villagers of Estwald
- Hanweir Watchkeep
- Instigator Gang
- Grizzled Outcasts
- Ulvenwald Mystics
- Kruin Outlaw

## Tests

### tormented_pariah_front_trigger_not_queued_when_one_spell_cast
Scenario: Tormented Pariah (front face) is on the battlefield; exactly one spell was cast last turn; the upkeep trigger should NOT appear on the stack because the intervening-if condition ('no spells were cast last turn') is false.

### rampaging_werewolf_back_trigger_not_queued_when_one_spell_cast
Scenario: Tormented Pariah is transformed to Rampaging Werewolf; exactly one spell was cast last turn; the upkeep trigger should NOT appear on the stack because the intervening-if condition ('a player cast two or more spells last turn') is false.

