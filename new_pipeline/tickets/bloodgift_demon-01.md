---
id: bloodgift_demon-01
status: fixed
card: Bloodgift Demon
audit_run_id: 2026-04-19-bloodgift_demon-audit
audit_model: sonnet
audit_tokens: 13532
audit_duration: 493
fixed_sha: bfcb01c57a8d3097e9b8904ac6da8984fcc6ff81
fixed_at: 2026-08-23T20:33:49Z
test_file: mtg-engine/tests/trigger_targets_declared.rs
fix_note: cluster fix: declares target_requirement so the engine chooses targets at stack-push time (CR 603.3b/c, 608.2b)
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, target player draws a card and loses 1 life.

**Code:**
> TriggeredAbilityDef {
    kind: TriggerKind::Upkeep,
    description: "target player draws a card and loses 1 life".into(),
    target_requirement: None,
},

// in on_upkeep:
fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {

**Description:**
The card's TriggeredAbilityDef declares `target_requirement: None`, causing the engine to classify the upkeep trigger as untargeted. `process_pending_trigger_pushes` in triggers.rs reads `target_requirement` and, finding None, pushes the trigger onto the stack immediately without selecting a target. The `on_upkeep` handler then compensates by manually constructing a ChooseTarget prompt at resolution time, ignoring the `_chosen_targets` parameter entirely. This violates CR 603.3b, which requires that targets for triggered abilities be chosen when the trigger is put on the stack — not when it resolves. Three downstream consequences follow: (1) the controller gains an information advantage, seeing how opponents respond to the trigger on the stack before committing to a target; (2) CR 603.3c is bypassed — because the trigger goes on the stack without any declared targets, the engine never evaluates whether legal targets exist at placement time, so a trigger with zero legal targets still reaches the stack; (3) the CR 608.2b target-legality re-check at resolution is also skipped, since the engine sees no declared targets on the resolved trigger. The fix is to change `target_requirement: None` to `target_requirement: Some(TargetRequirement::PlayerOnly)` and rewrite `on_upkeep` to consume the `chosen_targets` parameter rather than re-selecting targets. `TargetRequirement::PlayerOnly` is already defined in cards/mod.rs and is correctly handled by `valid_targets_for_req` in engine.rs, including proper hexproof filtering via `can_target_player`.

**Engine path:** mtg-engine/src/cards/isd/bloodgift_demon.rs:34

**Required check:** 8b

## Tests

### bloodgift_target_hexproof_gained_in_response
Scenario: Two-player game; demon controller's upkeep trigger goes on the stack; opponent responds by gaining hexproof. Under correct rules (CR 603.3b + 608.2b), the target was locked in at stack-placement time and the trigger fizzles at resolution since the chosen target now has hexproof. Under the current implementation, the controller chooses their target only at resolution and can re-target themselves instead of the now-hexproof opponent, so the trigger resolves on the controller rather than fizzling.

### bloodgift_trigger_never_stacks_no_legal_targets
Scenario: A multiplayer game in which every player other than the demon's controller has hexproof, and the controller also has hexproof; the trigger has zero legal targets under PlayerOnly semantics. Under correct rules (CR 603.3c), `process_pending_trigger_pushes` removes the trigger before it reaches the stack. Under the current implementation, the trigger always reaches the stack (target_requirement: None bypasses the zero-legal-targets check) and only fizzles silently inside on_upkeep when the computed targets list is empty.

