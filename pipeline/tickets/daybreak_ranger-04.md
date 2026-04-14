---
id: daybreak_ranger-04
status: new
card: Daybreak Ranger
card_file: mtg-engine/src/cards/isd/daybreak_ranger.rs
created: 2026-04-14T21:22:02Z
audit_run_id: 2026-04-14-daybreak_ranger-audit
audit_model: opus
audit_tokens: 12078
audit_duration: 368
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

**Code:**
> triggers.rs:817–861: Upkeep trigger dispatch creates `PendingTrigger::UpkeepTrigger` unconditionally for all permanents with `TriggerKind::Upkeep`, with no check of the intervening-if condition.

**Description:**
The oracle text uses an intervening-if clause: "At the beginning of each upkeep, **if** no spells were cast last turn, transform this creature." Per CR 603.4, both the trigger event and the if-clause must be true when the ability would trigger; otherwise the ability does not trigger at all (it does not go on the stack). The engine dispatches the upkeep trigger unconditionally and only checks the condition during resolution (in `on_upkeep` at daybreak_ranger.rs:151 via `should_transform`). This means the transform trigger goes on the stack and grants priority even when the intervening-if condition is false. While the resolution check prevents incorrect transforms, the trigger's presence on the stack is itself observable game state — it affects priority passes and interactions with cards that care about triggers being put on the stack.

**Engine path:**
- triggers.rs:817–861 (upkeep trigger dispatch — no intervening-if filter)
- daybreak_ranger.rs:147–158 (`on_upkeep` — checks condition at resolution only)

**Required check:** 8b

**Affected cards:**
- Daybreak Ranger / Nightfall Predator
- All werewolves with intervening-if transform triggers
- Any card with an intervening-if clause on an upkeep trigger

