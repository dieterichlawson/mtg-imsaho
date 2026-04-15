---
id: mayor_of_avabruck-02
status: deduped
card: Mayor of Avabruck
card_file: mtg-engine/src/cards/isd/mayor_of_avabruck.rs
created: 2026-04-14T20:57:26Z
audit_run_id: 2026-04-14-mayor_of_avabruck-audit
audit_model: opus
audit_tokens: 15497
audit_duration: 426
deduped_into: merged-intervening-if-01
---

## Audit Finding

**Oracle text:**
> "At the beginning of each upkeep, if no spells were cast last turn, transform this creature."
> "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."

**Code:**
> triggers.rs `GameEvent::StepStarted` handler (~lines 815-862): creates `PendingTrigger::UpkeepTrigger` for ALL permanents with `TriggerKind::Upkeep` unconditionally, without checking the intervening-if condition. The condition is only checked at resolution in `on_upkeep` (mayor_of_avabruck.rs:118).

**Description:**
Both transform abilities use intervening-if clauses ("if no spells were cast last turn" / "if a player cast two or more spells last turn"). Per CR 603.4, a triggered ability with an intervening-if clause triggers only if the condition is true when the trigger event occurs, and the effect happens only if the condition is still true when the trigger resolves. The engine checks the condition at resolution (correct) but not at trigger-creation time (incorrect). The trigger goes on the stack even when the condition is false. This is observable: opponents see it, can respond to it, and "whenever a triggered ability triggers" effects see it. It can also be countered by Stifle/Disallow when it shouldn't exist to counter.

**Engine path:**
- triggers.rs:815-862 — `StepStarted` handler creates triggers unconditionally
- mayor_of_avabruck.rs:114-125 — `on_upkeep` checks condition at resolution only

**Required check:** 8b

**Affected cards:**
- Mayor of Avabruck // Howlpack Alpha
- All Innistrad werewolves with intervening-if transform conditions
- Any card with an intervening-if triggered ability using the engine's trigger dispatch
