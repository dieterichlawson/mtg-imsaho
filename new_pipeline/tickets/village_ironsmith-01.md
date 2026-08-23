---
id: village_ironsmith-01
status: new
card: Village Ironsmith
audit_run_id: 2026-04-19-village_ironsmith-audit
audit_model: sonnet
audit_tokens: 25245
audit_duration: 494
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

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
Per CR 603.4, an intervening-if triggered ability ('At [event], if [condition], [effect]') must check the condition when the trigger event occurs, and triggers only if the condition is true at that moment. Both Village Ironsmith's front-face trigger ('if no spells were cast last turn') and Ironfang's back-face trigger ('if a player cast two or more spells last turn') are intervening-if clauses. The step-trigger dispatch in triggers.rs creates a PendingTrigger::UpkeepTrigger for any permanent whose current face has a non-empty upkeep trigger description, with no check of the card-specific condition. The condition is only checked at resolution time inside on_upkeep via should_transform(). This means both triggers appear on the stack every upkeep regardless of the spell-count condition, granting priority to players when it should not exist and letting them observe and respond to triggers that the rules say should never be created.

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 6

**Affected cards:**
- Mayor of Avabruck
- Daybreak Ranger
- Tormented Pariah
- Kruin Outlaw
- Instigator Gang
- Villagers of Estwald
- Hanweir Watchkeep
- Grizzled Outcasts
- Gatstaf Shepherd
- Reckless Waif
- Ulvenwald Mystics
- Screeching Bat

## Tests

### front_face_no_trigger_when_spells_were_cast
Scenario: Village Ironsmith is on the battlefield on its front face; one or more spells were cast last turn — no upkeep trigger should appear on the stack, but the engine creates one unconditionally.

### back_face_no_trigger_when_too_few_spells
Scenario: Ironfang (transformed Village Ironsmith) is on the battlefield; no single player cast two or more spells last turn — no upkeep trigger should appear on the stack, but the engine creates one unconditionally.

