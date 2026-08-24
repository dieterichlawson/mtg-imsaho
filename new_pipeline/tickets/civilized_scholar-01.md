---
id: civilized_scholar-01
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
> if let Some(obj) = state.get_object_mut(self_id) {
    obj.tapped = true; // "tap Homicidal Brute, then transform it"
    obj.is_transformed = false;
    obj.name = "Civilized Scholar".into();
}

**Description:**
The on_end_step handler transforms Homicidal Brute back to Civilized Scholar by directly mutating obj.is_transformed and obj.name without calling helpers::apply_transform(). The apply_transform helper also updates obj.subtypes and obj.keywords to match the new active face. Because apply_transform is skipped, obj.subtypes remains as ["Human", "Mutant"] (Homicidal Brute's subtypes) after the transform-back, instead of being reset to ["Human", "Advisor"] (Civilized Scholar's subtypes). In state.rs, the matches_filter function for CreatureFilter::HasSubtype runs an unconditional final fallback — creature.subtypes.iter().any(|s| s == subtype) at line 873 — after the registry-based check. When is_transformed is false, the registry check correctly returns false for "Mutant" (front-face registry data is ["Human", "Advisor"]), but control then falls through to the object-level fallback, which finds "Mutant" in the stale obj.subtypes and returns true. The result is that Civilized Scholar incorrectly reports being a Mutant after the end-step transform. The fix is to replace the manual obj.is_transformed/obj.name mutation with helpers::apply_transform(state, self_id, registry) and then separately set obj.tapped = true, analogous to how on_discard_choice already handles the forward transform.

**Engine path:** mtg-engine/src/cards/isd/civilized_scholar.rs:207

**Required check:** 8a

## Tests

### scholar_loses_mutant_subtype_after_end_step_transform
Scenario: Homicidal Brute fails to attack and transforms back to Civilized Scholar via the end-step trigger; verify via state.matches_filter that the creature no longer matches CreatureFilter::HasSubtype("Mutant") and still correctly matches CreatureFilter::HasSubtype("Advisor") after the transform.

