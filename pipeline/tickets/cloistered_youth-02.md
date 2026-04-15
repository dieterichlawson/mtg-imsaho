---
id: cloistered_youth-02
status: closed-duplicate
card: Cloistered Youth
card_file: mtg-engine/src/cards/isd/cloistered_youth.rs
created: 2026-04-15T03:43:08Z
audit_run_id: 2026-04-14-cloistered_youth-audit
audit_model: opus
audit_tokens: 11289
audit_duration: 242
duplicate_of: merged-your-upkeep-scope-02
---

## Audit Finding

**Oracle text:**
> "At the beginning of your upkeep, you may transform this creature."
> "At the beginning of your end step, you lose 1 life."

**Code:**
> `collect_triggers` (triggers.rs:823-861) creates UpkeepTrigger/EndStepTrigger for ALL permanents with a matching TriggerKind, regardless of whether it is the controller's step. Both `ap_triggers` and `nap_triggers` are pushed to the stack (triggers.rs:1122-1123).

**Description:**
The trigger dispatch creates upkeep and end step triggers for every permanent that declares the corresponding TriggerKind, during every player's upkeep/end step — not just the controller's. The card's `on_upkeep` (line 78) and `on_end_step` (line 112) gate on `active_player == controller` and return early, so the trigger resolves as a no-op. However, the phantom trigger is observably on the stack: opponents see it, priority passes occur around it, and players could respond to it (e.g., by casting instants). Per CR 603.2, "your upkeep" is part of the trigger condition — the trigger should not be created at all during the opponent's upkeep. This wastes game actions and exposes incorrect game state.

**Engine path:**
- triggers.rs:823-861 (StepStarted dispatch — creates triggers for all permanents)
- triggers.rs:1122-1123 (both AP and NAP trigger lists pushed to stack)
- cloistered_youth.rs:78-80, 112-114 (card handler early-return during wrong player's step)

**Required check:** 8b

**Affected cards:**
- Cloistered Youth // Unholy Fiend
- All cards with "your upkeep" / "your end step" triggers (engine-wide issue)

## Tests

### cloistered_youth_no_phantom_upkeep_trigger
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A controls Cloistered Youth (untransformed). During Player B's upkeep, verify that NO upkeep trigger for Cloistered Youth is placed on the stack. The trigger should only appear during Player A's upkeep.

### unholy_fiend_no_phantom_end_step_trigger
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A controls Unholy Fiend (transformed Cloistered Youth). During Player B's end step, verify that NO end step trigger for Unholy Fiend is placed on the stack, and Player A does NOT lose 1 life. The trigger should only appear during Player A's end step.
