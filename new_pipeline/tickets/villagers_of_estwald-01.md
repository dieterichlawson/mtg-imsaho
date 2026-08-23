---
id: villagers_of_estwald-01
status: fixed
card: Villagers of Estwald
audit_run_id: 2026-04-19-villagers_of_estwald-audit
audit_model: sonnet
audit_tokens: 29158
audit_duration: 548
fixed_sha: 28755d7786c3882a8061e402a59c15fd2378da86
fixed_at: 2026-08-23T17:03:38Z
test_file: mtg-engine/tests/intervening_if.rs
fix_note: cluster fix: CardBehavior::should_trigger gates dispatch on the intervening-if condition (CR 603.4)
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
            object_id: obj_id, card_id, controller, description: desc, chosen_targets: Vec::new(),
        },

**Description:**
Both the front-face trigger ('if no spells were cast last turn') and the back-face trigger ('if a player cast two or more spells last turn') are intervening-if clauses per CR 603.4. CR 603.4 requires the condition to be true at the moment the trigger event occurs — the trigger should only be placed on the stack when the condition is satisfied when the upkeep begins. The StepStarted dispatch in triggers.rs (around line 883) creates a UpkeepTrigger for every battlefield permanent whose active face returns a non-empty description for TriggerKind::Upkeep, without evaluating any intervening-if condition. The condition is only checked inside on_upkeep at resolution time via should_transform. When the condition is false — spells were cast last turn for the front face, or no single player cast 2+ spells for the back face — the trigger still appears on the stack, granting players an incorrect priority window and exposing observable intermediate game state that does not exist in a rules-correct implementation.

**Engine path:** mtg-engine/src/triggers.rs:883

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger
- Reckless Waif
- Tormented Pariah
- Gatstaf Shepherd
- Village Ironsmith
- Kruin Outlaw
- Mayor of Avabruck
- Grizzled Outcasts
- Ulvenwald Mystics
- Instigator Gang
- Hanweir Watchkeep

## Tests

### front_face_trigger_absent_when_spell_cast_last_turn
Scenario: Villagers of Estwald is on the battlefield in its front face; a spell was cast the previous turn; at the start of the next upkeep, no UpkeepTrigger for the front-face transform should appear on the stack, but it does.

### back_face_trigger_absent_when_no_player_cast_two_spells
Scenario: Howlpack of Estwald is on the battlefield (back face); no player cast two or more spells last turn; at the start of the next upkeep, no UpkeepTrigger for the back-face transform should appear on the stack, but it does.

