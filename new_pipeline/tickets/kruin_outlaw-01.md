---
id: kruin_outlaw-01
status: fixed
card: Kruin Outlaw
audit_run_id: 2026-04-19-kruin_outlaw-audit
audit_model: sonnet
audit_tokens: 29359
audit_duration: 497
fixed_sha: 28755d7786c3882a8061e402a59c15fd2378da86
fixed_at: 2026-08-23T17:03:38Z
test_file: mtg-engine/tests/intervening_if.rs
fix_note: cluster fix: CardBehavior::should_trigger gates dispatch on the intervening-if condition (CR 603.4)
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Code:**
> let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
if !desc.is_empty() {
    // ... PendingTrigger::UpkeepTrigger { ... } unconditionally pushed

**Description:**
Per CR 603.4, a triggered ability with an intervening-if clause ('At [event], if [condition], [effect]') must check the condition at the time the trigger event occurs; the trigger only goes on the stack if the condition is true at that moment. The engine's StepStarted handler (triggers.rs ~lines 836–880) calls `face_trigger_description` to decide whether to queue an upkeep trigger, but `face_trigger_description` only checks whether the card has an upkeep trigger at all — it never calls `should_transform` or otherwise evaluates the spell-count condition. As a result, both faces' transform triggers are placed on the stack at the beginning of every upkeep regardless of whether the condition is met. The condition is only checked at resolution time inside `on_upkeep` (kruin_outlaw.rs:102). This means players see a 'transform' trigger sitting on the stack — and have an opportunity to respond to it — even in turns where no transform will occur (e.g., front face when spells were cast last turn, back face when no player cast 2+ spells). This is an observable rules violation: the trigger should never have been on the stack.

**Engine path:** mtg-engine/src/triggers.rs:836

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger
- Instigator Gang
- Reckless Waif
- Gatstaf Shepherd
- Tormented Pariah
- Village Ironsmith
- Ulvenwald Mystics
- Villagers of Estwald
- Grizzled Outcasts
- Hanweir Watchkeep
- Mayor of Avabruck

## Tests

### front_face_transform_trigger_absent_when_spells_cast
Scenario: Front face (Kruin Outlaw) is on the battlefield; at least one spell was cast last turn — no transform trigger should be placed on the stack at the beginning of upkeep, but currently the trigger appears and fizzles.

### back_face_transform_trigger_absent_when_insufficient_spells
Scenario: Back face (Terror of Kruin Pass) is on the battlefield; no player cast 2 or more spells last turn — no transform-back trigger should appear on the stack at the beginning of upkeep, but currently the trigger appears and fizzles.

