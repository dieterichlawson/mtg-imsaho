---
id: grimgrin_corpse_born-02
status: new
card: Grimgrin, Corpse-Born
audit_run_id: 2026-04-19-grimgrin_corpse_born-audit
audit_model: sonnet
audit_tokens: 34446
audit_duration: 577
---

## Audit Finding

**Oracle text:**
> Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.

**Code:**
>             // Add +1/+1 counter to the source permanent.
            state.add_counters(*source_id, crate::types::CounterType::PlusOnePlusOne, 1);

**Description:**
`DestroyThenCounter` in engine.rs calls `add_counters(source_id, ...)` without checking whether `source_id` is still on the battlefield. If Grimgrin is destroyed in response to its own attack trigger (after the trigger has a chosen target), the trigger can still resolve (triggers stay on the stack after their source leaves play). `on_attacks` is dispatched without a zone guard (unlike many other `AttacksTrigger` handlers), calls `apply_pending_effect` with `DestroyThenCounter`, destroys the target normally, and then calls `add_counters` on Grimgrin's graveyard object. Per CR 122.1, counters can only meaningfully be placed on permanents. Worse, `move_object` clears counters only when an object *leaves* the battlefield, not when it *enters* from a non-battlefield zone (lines 586–595 of state.rs run only when `from == Zone::Battlefield`). So if Grimgrin's graveyard object has a counter planted on it by the stale trigger, and Grimgrin is later reanimated via Unburial Rites or similar, it enters the battlefield with a spurious +1/+1 counter, violating CR 400.7 (new object). The fix is to guard the `add_counters` call with a zone check: skip the counter if `source_id` is no longer on the battlefield.

**Engine path:** mtg-engine/src/engine.rs:3757

**Required check:** 8j

## Tests

### attack_trigger_no_counter_when_grimgrin_dies_in_response
Scenario: Grimgrin attacks with a valid target; in response, the opponent destroys Grimgrin; the attack trigger resolves (target is still legal), destroys the defending creature, but should NOT place a +1/+1 counter (Grimgrin is no longer on the battlefield)

### no_spurious_counter_after_die_trigger_reanimate
Scenario: Grimgrin attacks; opponent destroys Grimgrin in response; trigger resolves and erroneously places counter on graveyard Grimgrin; Grimgrin is reanimated with Unburial Rites; Grimgrin should enter as a 5/5 (no prior counters), not a 6/6

