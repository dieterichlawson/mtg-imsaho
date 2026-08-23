---
id: elder_cathar-01
status: new
card: Elder Cathar
audit_run_id: 2026-04-19-elder_cathar-audit
audit_model: sonnet
audit_tokens: 16530
audit_duration: 274
---

## Audit Finding

**Oracle text:**
> put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.

**Code:**
> TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "put +1/+1 counters on target creature".into(),
                target_requirement: None,
                },

**Description:**
The oracle text says 'target creature you control,' making this a targeted triggered ability. The TriggeredAbilityDef at elder_cathar.rs:29-33 declares target_requirement: None, causing process_pending_trigger_pushes (triggers.rs:1219-1222) to push the trigger onto the stack as untargeted with no target selected. Target selection is then deferred to on_dies at resolution time (lines 41-80). This violates three rules simultaneously. (1) CR 603.3b: targets for a triggered ability must be chosen when the trigger is put on the stack, not at resolution — opponents do not know what will be targeted when they decide whether to respond. (2) CR 603.3c: a triggered ability with no legal targets must not go on the stack at all; the inline early-return at line 46-48 silently does nothing but the trigger has already appeared on the stack, granting an incorrect priority window. (3) The inline target-list builder at lines 41-44 filters by zone, controller, and power but never calls can_be_targeted_by, so creatures with shroud are included as valid auto-selected or presented targets despite being untargetable by any ability (CR 702.18). The CR 608.2b re-check in resolve_next_trigger (triggers.rs:1298-1323) is also bypassed because it only fires when chosen_targets is non-empty, which it never is for this trigger. The correct fix is to declare target_requirement: Some(TargetRequirement::Creature), add is_valid_target restricting to creatures the controller controls, and update on_dies to read from the pre-selected chosen_targets rather than doing inline targeting.

**Engine path:** mtg-engine/src/cards/isd/elder_cathar.rs:32

**Required check:** 8f

**Affected cards:**
- Selhoff Occultist

## Tests

### elder_cathar_shroud_creature_auto_targeted
Scenario: Elder Cathar dies; the only creature the controller has on the battlefield has shroud — the trigger should be removed from consideration (no legal targets, CR 603.3c), but instead the counter is auto-applied to the shroud creature.

### elder_cathar_no_creatures_trigger_on_stack
Scenario: Elder Cathar dies with no other creatures on the battlefield; the trigger should not appear on the stack (CR 603.3c), but it does, giving opponents an undeserved priority window before the trigger resolves with no effect.

### elder_cathar_target_chosen_before_response_window
Scenario: Elder Cathar dies with two creatures on the battlefield; per CR 603.3b the target must be chosen as the trigger goes on the stack, but the current code pushes an untargeted trigger onto the stack first, then presents the target choice during resolution — opponents can incorrectly respond to the trigger before the target is known.

