---
id: hanweir_watchkeep-02
status: closed-duplicate
card: Hanweir Watchkeep
card_file: mtg-engine/src/cards/isd/hanweir_watchkeep.rs
created: 2026-04-14T21:30:20Z
audit_run_id: 2026-04-14-hanweir_watchkeep-audit
audit_model: opus
audit_tokens: 18615
audit_duration: 468
duplicate_of: merged-intervening-if-01
---

## Audit Finding

**Oracle text:**
> "At the beginning of each upkeep, if no spells were cast last turn, transform this creature."
> "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."

**Code:**
> Trigger dispatch (triggers.rs:822-841): collects all permanents with upkeep triggers and creates `PendingTrigger::UpkeepTrigger` for each, with no condition check. The intervening-if condition is only checked at resolution time in `on_upkeep` (hanweir_watchkeep.rs:89-99) via `should_transform`.

**Description:**
Both faces of this card have intervening-if triggered abilities (CR 603.4): the "if" clause between the trigger event and the effect. Per CR 603.4, the condition must be true when the trigger event occurs for the ability to trigger at all — "The ability triggers only if it is true—otherwise it does nothing." The engine's upkeep trigger dispatch (triggers.rs:822-865) collects ALL permanents with upkeep triggers and places them on the stack without evaluating any intervening-if condition. The condition is checked only at resolution. While the final game state is correct for this card (because `num_spells_cast_last_turn` doesn't change mid-turn, so the resolution check gives the same answer), the trigger incorrectly appears on the stack when the condition is false. The stack is publicly visible game state — opponents can observe the trigger and respond to it, even though by rule it should never have triggered.

**Engine path:**
- triggers.rs:815-865 (upkeep trigger collection — no intervening-if check)
- triggers.rs:1306-1311 (upkeep trigger resolution — zone check only)
- hanweir_watchkeep.rs:89-99 (`on_upkeep` — condition checked here)
- hanweir_watchkeep.rs:12-19 (`werewolf_should_transform` — the intervening-if condition)

**Required check:** 8b

**Affected cards:**
- Hanweir Watchkeep // Bane of Hanweir
- All ISD werewolves with intervening-if upkeep triggers
- Any card with an intervening-if upkeep trigger
