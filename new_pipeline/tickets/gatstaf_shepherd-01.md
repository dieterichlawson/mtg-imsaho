---
id: gatstaf_shepherd-01
status: fixed
card: Gatstaf Shepherd
audit_run_id: 2026-04-19-gatstaf_shepherd-audit
audit_model: sonnet
audit_tokens: 14847
audit_duration: 250
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

**Description:**
Both faces carry CR 603.4 intervening-if clauses: the front face only transforms 'if no spells were cast last turn' and the back face only transforms 'if a player cast two or more spells last turn'. Per CR 603.4, the condition must be true at the moment the triggering event occurs for the trigger to be placed on the stack at all. The dispatch at triggers.rs:843 creates a PendingTrigger::UpkeepTrigger for every battlefield permanent whose active face has a non-empty trigger description; the only guards are 'does this face have any upkeep trigger?' and the scope (Your vs Each). Neither `should_transform` nor any equivalent condition check is called at trigger-creation time. The condition is only evaluated later at resolution inside `on_upkeep`. As a result, both upkeep triggers are placed on the stack every upkeep regardless of last-turn spell count, giving all players a spurious priority window to cast instants or activate abilities in response to a trigger that will do nothing on resolution.

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger
- Reckless Waif
- Village Ironsmith
- Tormented Pariah
- Kruin Outlaw
- Instigator Gang
- Villagers of Estwald
- Hanweir Watchkeep
- Grizzled Outcasts
- Mayor of Avabruck
- Ulvenwald Mystics

## Tests

### gatstaf_shepherd_front_trigger_skipped_when_spell_cast
Scenario: Gatstaf Shepherd is on the battlefield (front face); a spell was cast last turn; verify the upkeep trigger does NOT appear on the stack.

### gatstaf_howler_back_trigger_skipped_when_one_spell_per_player
Scenario: Gatstaf Howler (back face) is on the battlefield; each player cast exactly one spell last turn (no player cast two or more); verify the upkeep trigger does NOT appear on the stack.

