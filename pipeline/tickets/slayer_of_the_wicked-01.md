---
id: slayer_of_the_wicked-01
status: new
card: Slayer of the Wicked
card_file: mtg-engine/src/cards/isd/slayer_of_the_wicked.rs
created: 2026-04-15T03:45:06Z
audit_run_id: 2026-04-14-slayer_of_the_wicked-audit
audit_model: opus
audit_tokens: 19326
audit_duration: 360
---

## Audit Finding

**Oracle text:**
> When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.

**Code:**
> slayer_of_the_wicked.rs:30 — `target_requirement: None`
> slayer_of_the_wicked.rs:41-51 — manual target enumeration in `on_enter_battlefield` iterates battlefield objects, checks subtypes, but never calls `can_be_targeted_by()`

**Description:**
The card's triggered ability declares `target_requirement: None`, causing the engine to push the ETB trigger onto the stack without selecting a target (triggers.rs:1163-1166). Target selection is instead performed manually during trigger resolution in `on_enter_battlefield` (lines 41-51). This violates CR 603.3d (targets for triggered abilities are chosen when the trigger goes on the stack) and CR 603.3c (a targeted trigger with no legal targets should not go on the stack). Additionally, the manual target enumeration skips `can_be_targeted_by()`, so hexproof (CR 702.11), shroud (CR 702.18), protection (CR 702.16), and ward (CR 702.21) are all ignored when building the target list. The engine already supports this card's targeting pattern: `CreatureWithFilter(SubtypeOrCardType { subtypes: ["Vampire", "Werewolf", "Zombie"], card_types: [] })` is the appropriate target requirement (cf. Urgent Exorcism's use of `PermanentWithFilter(SubtypeOrCardType { ... })` at urgent_exorcism.rs:30), combined with an `is_valid_target` implementation. Fixing the `target_requirement` to use this filter would simultaneously fix targeting timing AND hexproof/protection enforcement, since `process_pending_trigger_pushes` (triggers.rs:1143) calls `valid_targets_for_req` which includes `can_be_targeted_by`.

**Engine path:**
- mtg-engine/src/cards/isd/slayer_of_the_wicked.rs:30 (`target_requirement: None`)
- mtg-engine/src/cards/isd/slayer_of_the_wicked.rs:41-51 (manual target building without `can_be_targeted_by`)
- mtg-engine/src/triggers.rs:1163-1166 (untargeted triggers pushed directly onto stack)
- mtg-engine/src/triggers.rs:1176-1178 (`valid_targets_for_req` call for targeted triggers — the correct path this card should use)

**Required check:** 8b, 8f

**Affected cards:**
- Slayer of the Wicked
- Any other card using `target_requirement: None` with manual targeting in its ETB handler (pattern search recommended)

## Tests

### hexproof_zombie_not_targetable
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place a Zombie token with hexproof on the opponent's battlefield (e.g., create a Zombie token, then grant it hexproof via `obj.keywords`). Enter Slayer of the Wicked onto the controller's battlefield. Verify that the hexproof Zombie does NOT appear in the ETB trigger's target options. If no other valid targets exist, the trigger should not go on the stack at all (CR 603.3c).

### no_trigger_on_stack_without_valid_targets
Source ticket: (new)
Implementation: (not yet written)
Scenario: Enter Slayer of the Wicked onto the battlefield when no Vampire, Werewolf, or Zombie exists on the battlefield. Verify that no ETB trigger is placed on the stack (currently the trigger goes on the stack because `target_requirement: None` bypasses the CR 603.3c check in `process_pending_trigger_pushes`). After processing triggers, the stack should remain empty.

