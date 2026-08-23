---
id: curse_of_oblivion-01
status: new
card: Curse of Oblivion
audit_run_id: 2026-04-19-curse_of_oblivion-audit
audit_model: sonnet
audit_tokens: 13329
audit_duration: 233
---

## Audit Finding

**Oracle text:**
> At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.

**Code:**
> if behavior.step_trigger_scope(&kind, is_transformed) == crate::cards::TriggerScope::Your
    && controller != active_player
{
    continue;
}

**Description:**
The TriggerScope enum has only two values: Your (fires only during the controller's upkeep) and Each (fires during every player's upkeep). The oracle text requires the trigger to fire during the enchanted player's upkeep, which is typically a different player from the curse's controller. Because TriggerScope::Your would only fire on the controller's upkeep (wrong), the implementation falls back to the default TriggerScope::Each. Combined with the in-on_upkeep guard 'if state.active_player != cursed_player { return; }', the effect resolves correctly during the enchanted player's upkeep -- but the trigger still goes on the stack during every other player's upkeep as well, producing spurious stack entries. In a two-player game where Player A controls the curse on Player B, Player A's own upkeep will generate a trigger that appears on the stack, passes through priority, and resolves silently. This is an observable rules violation: players gain an incorrect priority window. The fix requires a new mechanism -- e.g. a TriggerScope::AttachedPlayer variant or a should_queue_step_trigger hook -- that checks attached_to_player == Some(active_player) at trigger-creation time. Curse of the Bloody Tome and Curse of the Pierced Heart use the identical pattern and are equally affected.

**Engine path:** mtg-engine/src/triggers.rs:885-888

**Required check:** 8b

**Affected cards:**
- Curse of the Bloody Tome
- Curse of the Pierced Heart

## Tests

### controller_upkeep_no_spurious_trigger
Scenario: Player A controls Curse of Oblivion enchanting Player B; during Player A's own upkeep no trigger should appear on the stack.

### enchanted_player_upkeep_exiles_two
Scenario: Player A controls Curse of Oblivion enchanting Player B who has 3+ cards in graveyard; during Player B's upkeep the trigger fires and Player B is asked to exile two cards.

