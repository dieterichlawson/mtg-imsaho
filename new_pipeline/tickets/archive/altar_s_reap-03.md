---
id: altar_s_reap-03
status: could_not_confirm
card: Altar's Reap
audit_run_id: 2026-04-18-altar_s_reap-audit
audit_model: sonnet
audit_tokens: 10071
audit_duration: 184
---

## Audit Finding

**Oracle text:**
> Players can only respond once this spell has been cast and all its costs have been paid. No one can try to destroy the creature you sacrificed to prevent you from casting this spell.

**Code:**
> // Pay additional costs (sacrifice) at cast time, before the spell goes on the stack.
if let Some(sac_id) = sacrifice {
    ...
    crate::destruction::sacrifice(&mut new_state, *sac_id, registry);
    ...
}
// ...
// Move to stack and store targets.
new_state.move_object(*object_id, Zone::Stack, registry);

**Description:**
The implementation correctly performs the creature sacrifice (lines ~2423-2445) before moving the spell to the stack (line 2524), matching the ruling that the creature is sacrificed as part of cost payment at 601.2h — before the spell exists on the stack and before opponents receive priority. However, no test explicitly verifies this ordering: no scenario checks that (a) the creature is already in the graveyard at the moment the spell first appears on the stack, and (b) the sacrifice is not deferred to resolution. Per the audit procedure for check 8j, the behavior appears correct but the ruling's key invariant is unexercised.

**Engine path:** mtg-engine/src/engine.rs:2422

**Required check:** 8j

## Tests

### altars_reap_creature_sacrificed_at_cast_time_not_resolution
Scenario: Cast Altar's Reap and inspect state immediately after `submit_action` (before resolving the top of stack): the sacrificed creature should already be in the graveyard, confirming the sacrifice occurred during cost payment and not during resolution.

## Test Run Results

- **altars_reap_creature_sacrificed_at_cast_time_not_resolution** — rejected
  - explanation: The code already correctly performs the sacrifice (engine.rs:2423-2446) before moving the spell to the stack (engine.rs:2525). A test was written that casts Altar's Reap with an explicit sacrifice, then asserts the creature is in the graveyard immediately after submit_action (before resolution). The test compiles and passes against current code, confirming the ordering invariant is already satisfied. No bug exists.

