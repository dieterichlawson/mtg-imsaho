---
id: splinterfright-01
status: new
card: Splinterfright
card_file: mtg-engine/src/cards/isd/splinterfright.rs
created: 2026-04-14T22:53:48Z
audit_run_id: 2026-04-14-splinterfright-audit
audit_model: opus
audit_tokens: 15601
audit_duration: 5057
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, mill two cards.

**Code:**
> triggers.rs:823-861 — `StepStarted { step: Upkeep }` creates `UpkeepTrigger` for ALL permanents on the battlefield with an Upkeep triggered ability, regardless of whose upkeep it is. No filter for `controller == active_player`.

**Description:**
Per CR 603.4, "at the beginning of your upkeep" means the trigger condition is "your upkeep begins." The trigger should only be created when the active player's upkeep starts AND the permanent's controller matches the active player. The dispatch at triggers.rs:823-861 creates an UpkeepTrigger for every permanent with a TriggerKind::Upkeep, then the card's `on_upkeep` handler (splinterfright.rs:58-59) early-returns with `if state.active_player != controller { return; }`. This creates a phantom trigger on the trigger queue during the opponent's upkeep that resolves to a no-op. Per CR 603.4, the trigger should never fire in the first place.

**Engine path:**
- triggers.rs:823-861 (dispatch creates trigger for all upkeeps without player filter)
- splinterfright.rs:58-59 (handler filters at resolution instead of dispatch)

**Required check:** 8b

**Affected cards:**
- Splinterfright
- All cards with `TriggerKind::Upkeep` that say "your upkeep" (Angel of Flight Alabaster, Bloodgift Demon, Charmbreaker Devils, etc.)

