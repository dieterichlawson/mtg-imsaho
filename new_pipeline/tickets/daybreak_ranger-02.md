---
id: daybreak_ranger-02
status: fixed
card: Daybreak Ranger
audit_run_id: 2026-04-19-daybreak_ranger-audit
audit_model: sonnet
audit_tokens: 29964
audit_duration: 567
fixed_sha: 28755d7786c3882a8061e402a59c15fd2378da86
fixed_at: 2026-08-23T17:03:38Z
test_file: mtg-engine/tests/intervening_if.rs
fix_note: cluster fix: CardBehavior::should_trigger gates dispatch on the intervening-if condition (CR 603.4)
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
--- Back Face ---
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Code:**
> let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
if !desc.is_empty() {
    // ... unconditionally creates PendingTrigger::UpkeepTrigger { ... }
}

**Description:**
The `StepStarted { step: Upkeep }` handler in `triggers.rs` (lines 876–918) creates a `PendingTrigger::UpkeepTrigger` for every battlefield permanent whose current face has a non-empty description for `TriggerKind::Upkeep`. It never evaluates the intervening-if condition before queuing. Per CR 603.4, the condition in 'At the beginning of each upkeep, **if** [condition]' must be true when the trigger event occurs; only then does the ability trigger and go on the stack. For Daybreak Ranger's front face, if spells were cast last turn, no trigger should appear on the stack at all — but one does, granting opponents a spurious priority window. For Nightfall Predator's back face, if no player cast two or more spells, the same spurious trigger appears. The condition is evaluated at resolution (in `on_upkeep` via `should_transform`), so the outcome is correct, but the phantom stack entry is itself observable game state.

**Engine path:** mtg-engine/src/triggers.rs:883

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger
- Nightfall Predator
- Hanweir Watchkeep
- Kruin Outlaw
- Mayor of Avabruck
- Reckless Waif
- Gatstaf Shepherd
- Grizzled Outcasts
- Villagers of Estwald
- Instigator Gang
- Ulvenwald Mystics

## Tests

### daybreak_ranger_no_phantom_transform_trigger_when_spell_cast
Scenario: Cast a spell during the current turn, then advance to the next upkeep. Verify that no UpkeepTrigger for Daybreak Ranger's front-face transform appears on the stack at all.

### nightfall_predator_no_phantom_transform_trigger_when_no_spells
Scenario: Daybreak Ranger transforms into Nightfall Predator. No spells are cast. Advance to the next upkeep. Verify no UpkeepTrigger for Nightfall Predator's back-face transform appears on the stack.

