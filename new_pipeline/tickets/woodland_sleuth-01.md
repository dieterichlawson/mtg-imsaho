---
id: woodland_sleuth-01
status: new
card: Woodland Sleuth
audit_run_id: 2026-04-19-woodland_sleuth-audit
audit_model: sonnet
audit_tokens: 17188
audit_duration: 291
---

## Audit Finding

**Oracle text:**
> Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.

**Code:**
> if let Some(behavior) = registry.get(card_id) {
    if behavior.has_etb_handler() {
        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::EntersBattlefield, false);
        let trigger = PendingTrigger::EnteredBattlefield {
            object_id: *object,
            card_id,
            controller,
            description: desc,
            chosen_targets: Vec::new(),
        };
        if controller == active_player {
            ap_triggers.push(trigger);
        } else {
            nap_triggers.push(trigger);
        }
    }
}

**Description:**
Per CR 603.4, a triggered ability reading 'When [event], if [condition], [effect]' must evaluate its intervening-if condition at trigger-creation time — if the condition is false, the trigger must not go on the stack at all. The ETB dispatch in `collect_triggers` (triggers.rs:565-580) creates a `PendingTrigger::EnteredBattlefield` for every card whose `has_etb_handler()` returns true, with no check of any intervening-if condition. When Woodland Sleuth enters the battlefield and no creature has died that turn (`state.creature_died_this_turn == false`), the trigger still goes on the stack, granting players an incorrect priority window. The resolution handler in `on_enter_battlefield` does check `state.creature_died_this_turn` and returns early when false, satisfying CR 603.4's second check, but the first check — which should suppress the trigger entirely — is absent from the dispatch. Hollowhenge Scavenger and Morkrut Banshee have the same structure (ETB handler that guards on `creature_died_this_turn` at resolution only) and share this bug.

**Engine path:** mtg-engine/src/triggers.rs:565

**Required check:** 8b

**Affected cards:**
- Hollowhenge Scavenger
- Morkrut Banshee

## Tests

### woodland_sleuth_no_morbid_trigger_suppressed
Scenario: When Woodland Sleuth enters the battlefield and no creature has died this turn, no ETB trigger should appear on the stack.

### woodland_sleuth_morbid_trigger_fires_and_returns_card
Scenario: When Woodland Sleuth enters after a creature has died this turn, the trigger goes on the stack, resolves, and returns a random creature card from the controller's graveyard to hand.

### woodland_sleuth_dies_in_response_can_return_itself
Scenario: When Woodland Sleuth enters with morbid active, an opponent kills the Sleuth in response to the trigger, and the trigger resolves — the Sleuth itself in the graveyard is eligible to be returned to hand at random.

