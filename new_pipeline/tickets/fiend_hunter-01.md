---
id: fiend_hunter-01
status: new
card: Fiend Hunter
audit_run_id: 2026-04-19-fiend_hunter-audit
audit_model: sonnet
audit_tokens: 39764
audit_duration: 801
---

## Audit Finding

**Oracle text:**
> When this creature enters, you may exile another target creature.

**Code:**
> // triggers.rs:573
chosen_targets: Vec::new(),

// fiend_hunter.rs:47-57
fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
    let controller = crate::cards::helpers::controller_of(state, object_id);
    let targets = crate::cards::helpers::creature_targets_except(state, object_id, object_id, controller, registry);
    crate::cards::helpers::present_optional_target_choice(
        state, object_id, controller, targets,
        PendingEffect::ExileAndStore { source_id: object_id, source_name: "Fiend Hunter".into() },
        "Fiend Hunter: you may exile another target creature",
    );
}

**Description:**
The ETB trigger dispatch in `collect_triggers` (triggers.rs:573) hardcodes `chosen_targets: Vec::new()` for every ETB trigger. Fiend Hunter's `on_enter_battlefield` ignores the `_chosen_targets` parameter entirely and instead gathers targets fresh from the current battlefield at resolution time via `present_optional_target_choice`. Per CR 603.3b, the controller must announce the target when the trigger is placed on the stack, not when it resolves. This creates two rules violations: (1) the Fiend Hunter controller gets to choose their target AFTER opponents have had priority and responded to the trigger, granting an illegitimate information advantage; (2) creatures that entered the battlefield after the trigger was created but before it resolved become valid targets, even though they should not have been legal at trigger-creation time. The ruling 'its first ability will resolve and exile the target creature indefinitely' further confirms that MTG presupposes a pre-committed target, since in the engine the controller can freely pick from the current battlefield at resolution time regardless of what was present when the trigger went on the stack.

**Engine path:** mtg-engine/src/triggers.rs:573, mtg-engine/src/cards/isd/fiend_hunter.rs:47

**Required check:** 8b

## Tests

### fiend_hunter_etb_target_locked_at_trigger_creation
Scenario: Fiend Hunter enters; opponent flash-plays a creature in response to the ETB trigger before it resolves; that creature should NOT be a valid exile target because it was not in play when the trigger was put on the stack

### fiend_hunter_etb_lifo_ruling_target_fixed
Scenario: Ruling-1 scenario: Fiend Hunter enters (ETB trigger created targeting creature A), then Fiend Hunter immediately leaves (LTB fires and does nothing); the ETB must resolve exiling creature A, not allow a fresh target selection from the current battlefield

