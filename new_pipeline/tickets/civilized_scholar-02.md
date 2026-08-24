---
id: civilized_scholar-02
status: fixed
card: Civilized Scholar
audit_run_id: 2026-04-19-civilized_scholar-audit
audit_model: sonnet
audit_tokens: 36063
audit_duration: 622
fixed_sha: 76d0ef84877d7dbd295f0f1fd8df00821e97f692
fixed_at: 2026-08-24T00:38:42Z
test_file: mtg-engine/tests/trigger_target_recheck.rs
fix_note: transform-back goes through apply_transform; the attack marker is turn-stamped so it cannot go stale
---

## Audit Finding

**Oracle text:**
> At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.

**Code:**
> // Clear the attack flag for next turn.
if let Some(obj) = state.get_object_mut(self_id) {
    obj.card_state.remove("attacked_this_turn");
}

**Description:**
The attacked_this_turn flag written to obj.card_state by on_attacks is only cleared inside on_end_step, which is only invoked when the creature is on the back face (Homicidal Brute) at the end step. The engine's Cleanup step handler does not clear card_state, and no other engine path resets per-turn flags stored there. If Civilized Scholar (front face) attacks in turn N but the creature stays on the front face through that turn's end step — possible because the EndStep trigger is registered only on the back face and is not collected for the front face — the flag is never cleared. In turn N+1 if the player activates the draw-discard ability, discards a creature card, and Civilized Scholar transforms into Homicidal Brute, the stale flag from turn N is still present. At the end of turn N+1, the EndStep trigger fires and on_end_step reads attacked = card_state.contains_key("attacked_this_turn") as true, incorrectly deciding the creature attacked this turn and skipping the tap-and-transform. Homicidal Brute therefore persists through the end step when it should have transformed back to Civilized Scholar. Note that the same-turn case covered by the 2011-09-22 ruling (Scholar attacks, then transforms to Brute in the same turn, then Brute's trigger fires) works correctly; the bug is the cross-turn case where the attack occurred on a prior turn while the creature was on the front face.

**Engine path:** mtg-engine/src/cards/isd/civilized_scholar.rs:215

**Required check:** 8j

## Tests

### stale_attacked_flag_not_blocking_transform_across_turns
Scenario: Turn 1: Civilized Scholar (front face) attacks and is not transformed during that turn's end step. Turn 2: player activates the draw-discard ability and discards a creature card, transforming to Homicidal Brute. Homicidal Brute does not attack in turn 2. At turn 2's end step, verify that Homicidal Brute correctly transforms back to Civilized Scholar (the stale flag from turn 1 must not prevent the transform).

