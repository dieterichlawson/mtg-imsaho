---
id: mentor_of_the_meek-01
status: new
card: Mentor of the Meek
card_file: mtg-engine/src/cards/isd/mentor_of_the_meek.rs
created: 2026-04-15T03:46:51Z
audit_run_id: 2026-04-14-mentor_of_the_meek-audit
audit_model: opus
audit_tokens: 19660
audit_duration: 465
---

## Audit Finding

**Oracle text:**
> Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.

**Code:**
> `mentor_of_the_meek.rs:50`: `let power = state.effective_power(entered_id, registry).unwrap_or(99);`
> This line runs inside `on_any_creature_enters`, which is called at trigger resolution time (triggers.rs:1269), not at trigger creation time (triggers.rs:564).

**Description:**
The power check happens at trigger resolution time instead of when the creature enters. The `EnterWatch` trigger is created unconditionally for all watchers in `collect_triggers` (triggers.rs:564) without recording the entered creature's power. When the trigger resolves and calls `on_any_creature_enters`, it reads the creature's current `effective_power` — which may have changed since entry (pumped, shrunk, or the creature may have left the battlefield entirely). Per ruling 1, the power is locked at entry time: once the trigger fires, subsequent power changes don't affect it. If the creature left the battlefield before resolution, `effective_power` returns `None` → defaults to 99 → trigger is incorrectly silenced, even though the creature entered with qualifying power and the trigger should still resolve.

**Engine path:**
- triggers.rs:564 — `EnterWatch` created without power snapshot
- triggers.rs:1269 — `on_any_creature_enters` called at resolution time
- mentor_of_the_meek.rs:50 — power check at resolution, not entry

**Required check:** 8b, 8j

**Affected cards:**
- Mentor of the Meek
- Any future card with AnyCreatureEnters that filters by a creature characteristic at entry time

## Tests

### mentor_pump_after_entry_still_triggers
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Mentor of the Meek on the battlefield. Enter a 2/2 creature. Before the EnterWatch trigger resolves, apply an until-end-of-turn +2/+2 effect to the entered creature (making it 4/4). Resolve the trigger. Assert that the YesNo pay choice IS presented — the creature had power 2 at entry time, so the trigger should fire regardless of its current power.

### mentor_creature_leaves_before_resolution
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Mentor of the Meek on the battlefield. Enter a 1/1 creature. Before the EnterWatch trigger resolves, move the 1/1 to the graveyard (e.g., destroy it). Resolve the trigger. Assert that the YesNo pay choice IS presented — the creature entered with power 1, so the trigger should fire even though the creature is no longer on the battlefield.

