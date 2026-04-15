---
id: mayor_of_avabruck-03
status: closed-duplicate
card: Mayor of Avabruck
card_file: mtg-engine/src/cards/isd/mayor_of_avabruck.rs
created: 2026-04-14T20:57:26Z
audit_run_id: 2026-04-14-mayor_of_avabruck-audit
audit_model: opus
audit_tokens: 15497
audit_duration: 426
duplicate_of: merged-your-upkeep-scope-02
---

## Audit Finding

**Oracle text:**
> "At the beginning of your end step, create a 2/2 green Wolf creature token."

**Code:**
> triggers.rs `GameEvent::StepStarted` handler (~lines 815-862): creates `PendingTrigger::EndStepTrigger` for ALL permanents with `TriggerKind::EndStep` during any end step. Both `ap_triggers` and `nap_triggers` are pushed to the stack (line 1123: `state.pending_trigger_pushes_nap.extend(nap_triggers)`).
> mayor_of_avabruck.rs:133: `if !is_transformed || state.active_player != controller { return; }` — guards resolution.

**Description:**
Howlpack Alpha's wolf-token ability says "your end step," meaning it should only trigger during the controller's end step. The engine fires `TriggerKind::EndStep` triggers for all permanents regardless of whose end step it is. During an opponent's end step, the trigger goes on the stack (via `nap_triggers`) but resolves to nothing because the card code checks `active_player != controller`. The functional outcome is correct (no token created), but the trigger incorrectly appears on the stack — it's observable, can be responded to, and can be countered when it shouldn't exist. The `TriggerKind` enum has no concept of "your" vs "each" step scope; all step triggers are treated as "each."

**Engine path:**
- triggers.rs:860 — nap_triggers pushed for non-active-player permanents
- triggers.rs:1123 — `nap_triggers` extended into `pending_trigger_pushes_nap`
- mayor_of_avabruck.rs:127-133 — `on_end_step` guards with active_player check

**Required check:** 8b

**Affected cards:**
- Howlpack Alpha (Mayor of Avabruck back face)
- All cards with "at the beginning of your end step/upkeep/draw step" that use `TriggerKind::EndStep`/`Upkeep` without engine-level scope filtering
