---
id: kruin_outlaw-01
status: deduped
card: Kruin Outlaw
card_file: mtg-engine/src/cards/isd/kruin_outlaw.rs
created: 2026-04-14T20:54:55Z
audit_run_id: 2026-04-14-kruin_outlaw-audit
audit_model: opus
audit_tokens: 13110
audit_duration: 275
deduped_into: merged-intervening-if-01
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

> At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Code:**
> In `triggers.rs:822-841`, the `StepStarted { step: Upkeep }` handler creates an `UpkeepTrigger` for every permanent whose current face has a `TriggerKind::Upkeep` triggered ability. No intervening-if condition is evaluated. The condition is only checked later in `on_upkeep` (kruin_outlaw.rs:102) via `should_transform()` at resolution time.

**Description:**
Both faces of Kruin Outlaw have intervening-if clauses ("if no spells were cast last turn" / "if a player cast two or more spells last turn"). Per CR 603.4, an intervening-if condition must be true at the moment the trigger event occurs for the ability to trigger at all — if the condition is false, the ability does not trigger and nothing goes on the stack. The current engine unconditionally places the trigger on the stack and defers the condition check to resolution. While functionally equivalent for this specific card (the spell-count-last-turn value cannot change during the current upkeep), this deviates from the CR-defined procedure: the stack contains a trigger object that should not exist, which is observable by other game effects that care about triggers on the stack (e.g., "Whenever an ability triggers" effects, or card count of stack objects). This is an engine-level limitation — the trigger dispatch system has no mechanism to evaluate intervening-if conditions at fire time.

**Engine path:**
- triggers.rs:822-841 (unconditional UpkeepTrigger creation)
- kruin_outlaw.rs:98-109 (condition check deferred to on_upkeep resolution)

**Required check:** 8b

**Affected cards:**
- Kruin Outlaw / Terror of Kruin Pass
- All ISD werewolves with intervening-if transform triggers (Reckless Waif, Mayor of Avabruck, Daybreak Ranger, Villagers of Estwald, etc.)
- Any card with an intervening-if triggered ability using TriggerKind::Upkeep, EndStep, or EndCombat
