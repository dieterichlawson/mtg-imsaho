---
id: ulvenwald_mystics-01
status: new
card: Ulvenwald Mystics
audit_run_id: 2026-04-19-ulvenwald_mystics-audit
audit_model: sonnet
audit_tokens: 14809
audit_duration: 281
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature. [back face:] At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

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
            ...
        },
        ...
    };
    // unconditionally pushed to ap_triggers / nap_triggers
}

**Description:**
Both upkeep triggers on Ulvenwald Mystics contain intervening-if clauses (CR 603.4): the front face fires only 'if no spells were cast last turn', and the back face fires only 'if a player cast two or more spells last turn'. Per CR 603.4, the condition must be true both when the trigger event occurs AND when the trigger resolves — if it is false at event time, the trigger must not be placed on the stack at all. The engine's `collect_triggers` function in triggers.rs tests only whether `face_trigger_description` returns a non-empty string (i.e., whether the card declares an upkeep trigger at all); it never evaluates the actual condition. Both triggers are therefore queued unconditionally every upkeep regardless of spell-cast history. The condition is checked a second time at resolution inside `on_upkeep` (via `should_transform`), so no spurious transform occurs — but the trigger still appears on the stack and players receive priority to respond, which is observable game state that should not exist when the condition is false. For example, on a turn where spells were cast, the front-face trigger still goes on the stack, letting players respond to a trigger that should never have been created.

**Engine path:** mtg-engine/src/triggers.rs:843

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger
- Instigator Gang
- Village Ironsmith
- Kruin Outlaw
- Reckless Waif
- Villagers of Estwald
- Tormented Pariah
- Mayor of Avabruck
- Hanweir Watchkeep
- Grizzled Outcasts
- Gatstaf Shepherd

## Tests

### front_face_trigger_fires_when_spell_was_cast
Scenario: A spell was cast last turn; Ulvenwald Mystics is on its front face; the upkeep trigger should not appear on the stack, but it does — engine queues it unconditionally.

### back_face_trigger_fires_when_zero_spells_cast
Scenario: No spells were cast last turn; Ulvenwald Primordials is on its back face; the back-face trigger ('if a player cast two or more spells') should not appear on the stack, but it does — engine queues it unconditionally.

### back_face_trigger_fires_when_one_spell_cast
Scenario: Exactly one spell was cast last turn by a single player; Ulvenwald Primordials is on its back face; the condition (2+ spells by any one player) is false, so the trigger should not appear — but it does.

