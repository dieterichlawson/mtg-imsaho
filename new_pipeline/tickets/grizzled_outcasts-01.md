---
id: grizzled_outcasts-01
status: fixed
card: Grizzled Outcasts
audit_run_id: 2026-04-19-grizzled_outcasts-audit
audit_model: sonnet
audit_tokens: 13985
audit_duration: 262
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
    if behavior.step_trigger_scope(&kind, is_transformed) == crate::cards::TriggerScope::Your
        && controller != active_player
    {
        continue;
    }
    let trigger = match kind {
        crate::cards::TriggerKind::Upkeep => PendingTrigger::UpkeepTrigger {

**Description:**
Both the front-face and back-face upkeep triggers of Grizzled Outcasts / Krallenhorde Wantons carry intervening-if clauses ('if no spells were cast last turn' and 'if a player cast two or more spells last turn'). Per CR 603.4, a triggered ability phrased 'At [event], if [condition], [effect]' may only be placed on the stack when the condition is true at the moment the triggering event occurs. The StepStarted handler in triggers.rs (lines 843-844) queues the upkeep trigger for any permanent whose face has a non-empty trigger description, with no evaluation of the intervening-if condition. The condition is only checked later at resolution inside on_upkeep via should_transform. As a result, on every upkeep — regardless of how many spells were cast last turn — one of these triggers goes onto the stack, giving players a spurious window to respond (cast instants, activate abilities) even in turns where no transform would occur. This affects both the front face (trigger queued even when spells were cast) and the back face Krallenhorde Wantons (trigger queued even when no single player cast two or more spells).

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 8b

**Affected cards:**
- Village Ironsmith
- Kruin Outlaw
- Reckless Waif
- Instigator Gang
- Tormented Pariah
- Daybreak Ranger
- Ulvenwald Mystics
- Villagers of Estwald
- Hanweir Watchkeep
- Gatstaf Shepherd
- Mayor of Avabruck

## Tests

### front_face_no_transform_trigger_not_queued_when_spell_cast
Scenario: Grizzled Outcasts (front face) is on the battlefield; active player cast one spell last turn — the upkeep trigger should NOT appear on the stack at all, but currently it does.

### back_face_no_transform_trigger_not_queued_when_one_spell_each
Scenario: Krallenhorde Wantons (back face) is on the battlefield; both players each cast exactly one spell last turn (total >= 2, but no single player cast two) — the upkeep trigger should NOT appear on the stack, but currently it does.

