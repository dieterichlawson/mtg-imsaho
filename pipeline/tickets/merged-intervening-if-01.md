---
id: merged-intervening-if-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: daybreak_ranger-04, hanweir_watchkeep-02, kruin_outlaw-01, mayor_of_avabruck-02
---

# Intervening-if conditions not evaluated at trigger fire time (CR 603.4)

## Description
Per CR 603.4, "an intervening-if condition — the clause that appears between the trigger event and the effect — must be true for the ability to trigger." If the condition is false when the trigger event occurs, the ability does not trigger at all and nothing is placed on the stack. The engine's step-started dispatch (`triggers.rs:815-862`) queues a PendingTrigger for every permanent with a matching `TriggerKind`, regardless of any intervening-if. The condition is evaluated only at resolution inside the card's handler, so a phantom trigger appears on the stack when it should not exist. The stack is observable game state — opponents can respond, Stifle can counter, "whenever a triggered ability triggers" effects see it — so the deviation is observable even when the final outcome is unchanged.

## Engine path
- triggers.rs:815-862 (step-started trigger dispatch — no intervening-if hook)
- triggers.rs:492 (face_trigger_description — returns description for any trigger kind regardless of condition)

## Tests

### test_daybreak_ranger_no_phantom_transform_trigger_when_spells_cast
Source ticket: daybreak_ranger-04
Implementation: (not yet written)
Scenario: Cast a spell this turn, then the next upkeep begins. Verify no UpkeepTrigger for Daybreak Ranger's transform appears on the stack.

### test_hanweir_watchkeep_no_phantom_transform_trigger
Source ticket: hanweir_watchkeep-02
Implementation: (not yet written)
Scenario: Hanweir Watchkeep is on the battlefield; a spell was cast last turn. At the beginning of upkeep, verify no transform trigger is placed on the stack.

### test_kruin_outlaw_no_phantom_transform_trigger
Source ticket: kruin_outlaw-01
Implementation: (not yet written)
Scenario: Kruin Outlaw is on the battlefield; a spell was cast last turn. At upkeep, verify no transform trigger is on the stack.

### test_mayor_of_avabruck_no_phantom_transform_trigger
Source ticket: mayor_of_avabruck-02
Implementation: (not yet written)
Scenario: Mayor of Avabruck is on the battlefield; a spell was cast last turn. At upkeep, verify no transform trigger is placed on the stack.

