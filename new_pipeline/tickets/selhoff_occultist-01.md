---
id: selhoff_occultist-01
status: fixed
card: Selhoff Occultist
audit_run_id: 2026-04-19-selhoff_occultist-audit
audit_model: sonnet
audit_tokens: 21123
audit_duration: 374
fixed_sha: bfcb01c57a8d3097e9b8904ac6da8984fcc6ff81
fixed_at: 2026-08-23T20:33:49Z
test_file: mtg-engine/tests/trigger_targets_declared.rs
fix_note: cluster fix: declares target_requirement so the engine chooses targets at stack-push time (CR 603.3b/c, 608.2b)
---

## Audit Finding

**Oracle text:**
> target player mills a card

**Code:**
> TriggeredAbilityDef {
    kind: TriggerKind::SelfDies,
    description: "target player mills a card".into(),
    target_requirement: None,
},
TriggeredAbilityDef {
    kind: TriggerKind::AnyCreatureDies,
    description: "target player mills a card".into(),
    target_requirement: None,
},

**Description:**
Both triggered ability definitions declare `target_requirement: None`, so `process_pending_trigger_pushes` treats them as untargeted and pushes them onto the stack immediately with `chosen_targets: Vec::new()`. Target selection then happens inside `on_dies` / `on_any_creature_dies` via `awaiting_action` (the `present_mill_choice` function). Per CR 603.3b, the controller of a triggered ability must choose targets when the trigger is put on the stack — not at resolution. The engine already supports correct target-selection-at-stack-placement for SelfDies and DeathWatch triggers: when `target_requirement` is `Some(TargetRequirement::PlayerOnly)`, `process_pending_trigger_pushes` calls `valid_targets_for_req`, auto-picks if there is exactly one legal target, or prompts the controller before pushing onto the stack. The card simply fails to declare the requirement. Deferring selection to resolution lets the controller gain information from opponent responses before committing to a target, and the CR 608.2b legality re-check at resolution is skipped because `chosen_targets` is empty.

**Engine path:** mtg-engine/src/cards/isd/selhoff_occultist.rs:31

**Required check:** 8b

## Tests

### self_dies_target_selected_at_stack_placement
Scenario: Selhoff Occultist dies; the controller should be prompted to choose the mill target before the trigger appears on the stack, not after it resolves.

### any_creature_dies_target_selected_at_stack_placement
Scenario: Another creature dies while Selhoff Occultist is on the battlefield; the controller should be prompted to choose the mill target when the DeathWatch trigger is put on the stack, not at resolution.

