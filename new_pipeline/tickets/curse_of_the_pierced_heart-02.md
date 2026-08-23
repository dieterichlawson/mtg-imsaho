---
id: curse_of_the_pierced_heart-02
status: fixed
card: Curse of the Pierced Heart
audit_run_id: 2026-04-19-curse_of_the_pierced_heart-audit
audit_model: sonnet
audit_tokens: 30078
audit_duration: 606
fixed_sha: bfcb01c57a8d3097e9b8904ac6da8984fcc6ff81
fixed_at: 2026-08-23T20:33:49Z
test_file: mtg-engine/tests/trigger_targets_declared.rs
fix_note: gates upkeep dispatch on the enchanted player via should_trigger instead of an inert stack entry
---

## Audit Finding

**Oracle text:**
> At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.

**Code:**
> if behavior.step_trigger_scope(&kind, is_transformed) == crate::cards::TriggerScope::Your
                                    && controller != active_player
                                {
                                    continue;
                                }

**Description:**
The upkeep-trigger dispatch in `collect_triggers` (triggers.rs:885) skips a permanent's upkeep trigger only when `step_trigger_scope` returns `TriggerScope::Your` AND the permanent's controller is not the active player. `CurseOfThePiercedHeart` does not override `step_trigger_scope`, so it uses the default `TriggerScope::Each` — causing the trigger to be queued at the beginning of EVERY player's upkeep, not just the enchanted player's. The card's `on_upkeep` handler guards with `if state.active_player != cursed_player { return; }` (line 56), which prevents the damage effect from firing on the wrong turn. However, by that point the trigger is already on the stack, granting both players a spurious priority window during any non-enchanted-player upkeep. Per CR 603.2, a triggered ability should be placed on the stack only when its trigger event occurs; 'at the beginning of enchanted player's upkeep' fires only when the enchanted player's upkeep begins. The engine's `TriggerScope` enum provides no value for 'fire only on a specific other player's upkeep', so the card cannot correct this without a new card-behavior hook (e.g., `should_queue_upkeep_trigger(state, id, registry) -> bool`). This same bug affects Curse of Oblivion, which uses an identical in-handler early-return pattern.

**Engine path:** mtg-engine/src/triggers.rs:885

**Required check:** 8b

**Affected cards:**
- Curse of Oblivion

## Tests

### curse_trigger_not_queued_on_non_enchanted_upkeep
Scenario: Curse of the Pierced Heart enchants player B; at the beginning of player A's (the Curse controller's) own upkeep, no trigger for the Curse should be placed on the stack — currently an upkeep trigger is queued and gives players a spurious priority window.

