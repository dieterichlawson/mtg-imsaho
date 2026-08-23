---
id: mentor_of_the_meek-01
status: new
card: Mentor of the Meek
audit_run_id: 2026-04-19-mentor_of_the_meek-audit
audit_model: sonnet
audit_tokens: 25250
audit_duration: 428
---

## Audit Finding

**Oracle text:**
> Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card. [Ruling] Mentor of the Meek's ability checks the power of the other creature only as it enters. If that creature's power is 2 or less, the ability will trigger. Once the ability triggers, raising that creature's power above 2 won't affect that ability. Similarly, reducing the creature's power to 2 or less after it enters won't cause the ability to trigger.

**Code:**
> // Check if the entering creature has power 2 or less.
        let power = state.effective_power(entered_id, registry).unwrap_or(99);
        if power > 2 {
            return;
        }

**Description:**
The power filter runs inside `on_any_creature_enters` (mentor_of_the_meek.rs:49–52), which executes at trigger RESOLUTION time, not at trigger CREATION time. The `PendingTrigger::EnterWatch` struct (triggers.rs:49–56) stores only `entered_id`, `entered_controller`, and the watcher's identity — it does not snapshot the entering creature's effective power when the trigger fires. The `collect_triggers` handler (triggers.rs:586–614) creates `EnterWatch` triggers for every creature entry with no power filter at creation. Two wrong outcomes result: (1) a creature enters with power ≤ 2 (trigger should fire and draw), then gets pumped above 2 before the trigger resolves — `effective_power` now returns the inflated value, the `power > 2` guard fires, and no draw happens even though the ruling says the trigger is already locked in; (2) a creature enters with power > 2 (trigger should NOT fire), then gets reduced to ≤ 2 before resolution — `effective_power` now returns the reduced value, the guard passes, and the player is incorrectly offered the {1} payment choice. The fix is to add an `entered_power: i32` field to `PendingTrigger::EnterWatch`, populate it with `state.effective_power(entered_id, registry)` at trigger creation in `collect_triggers`, and replace the resolution-time `effective_power` call in the card with a check against that stored value.

**Engine path:** mtg-engine/src/cards/isd/mentor_of_the_meek.rs:49

**Required check:** 8j

**Affected cards:**
- Champion of the Parish

## Tests

### mentor_power_check_at_entry_pump_before_resolution
Scenario: A 2/2 creature enters under Mentor's controller; before the Mentor trigger resolves an instant pumps that creature to 5/5. The trigger should still draw a card (power was ≤ 2 at entry), but currently the resolution-time power check returns 5 > 2 and no draw occurs.

### mentor_no_trigger_for_high_power_creature_reduced_before_resolution
Scenario: A 5/5 creature enters under Mentor's controller (power > 2 at entry, so no trigger should fire); before the spuriously-created EnterWatch trigger resolves an instant reduces it to 1/1. Currently the resolution-time power check returns 1 ≤ 2 and the player is incorrectly offered the {1} draw choice.

