---
id: gutter_grime-02
status: fixed
card: Gutter Grime
audit_run_id: 2026-04-19-gutter_grime-audit
audit_model: sonnet
audit_tokens: 42107
audit_duration: 797
fixed_sha: 5b2471bda7cbdf3ac83b8d6bf521bd75796fcdc6
fixed_at: 2026-08-23T23:34:08Z
test_file: mtg-engine/tests/trigger_independence.rs
fix_note: add_counters is a no-op off the battlefield (CR 121.1)
---

## Audit Finding

**Oracle text:**
> If Gutter Grime leaves the battlefield, the power and toughness of each Ooze token it created will become 0.

**Code:**
> let controller = match state.get_object(self_id) {
    Some(o) => o.controller,  // no zone check — runs even if self_id is in graveyard
    None => return,
};
// ...
state.add_counters(self_id, CounterType::Slime, 1);  // line 62

**Description:**
When Gutter Grime's AnyCreatureDies trigger is on the stack and Gutter Grime is destroyed in response (before the trigger resolves), `on_any_creature_dies` does not guard against Gutter Grime being off the battlefield. `state.add_counters(self_id, CounterType::Slime, 1)` is called unconditionally, and `add_counters` has no zone guard — it adds the counter directly to the graveyard object. Because `effective_power` / `effective_toughness` read slime counters from `get_object(source_id)` regardless of the source's zone, the graveyard Gutter Grime now reports 1 slime counter instead of 0. This causes the newly created Ooze token to have P/T 1/1 (instead of 0/0 as the ruling requires), so it survives state-based action checks rather than dying immediately. Additionally, if Gutter Grime is later reanimated, `move_object` only clears counters when an object *leaves* the battlefield, not when it *enters* — so the spurious slime counter persists on the object through the graveyard-to-battlefield transition, causing Gutter Grime to incorrectly enter the battlefield with 1 slime counter already on it.

**Engine path:** mtg-engine/src/cards/isd/gutter_grime.rs:62

**Required check:** 8j

## Tests

### ooze_token_is_zero_zero_when_gutter_grime_leaves_before_trigger_resolves
Scenario: Gutter Grime's trigger is on the stack; opponent destroys Gutter Grime in response; the trigger resolves and creates an Ooze token; that token should be 0/0 (and die to SBAs) but is instead 1/1.

### gutter_grime_reanimated_without_spurious_counter
Scenario: Gutter Grime's trigger resolves while Gutter Grime is in the graveyard (destroyed in response to its own trigger); Gutter Grime is later reanimated; it should enter the battlefield with 0 slime counters but incorrectly enters with 1.

