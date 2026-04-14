---
id: nevermore-01
status: new
card: Nevermore
card_file: mtg-engine/src/cards/isd/nevermore.rs
created: 2026-04-14T21:47:55Z
audit_run_id: 2026-04-14-nevermore-audit
audit_model: opus
audit_tokens: 8084
audit_duration: 1168
---

## Audit Finding

**Oracle text:**
> As this enchantment enters, choose a nonland card name.

**Code:**
> `nevermore.rs:33-38` declares `triggered_abilities: vec![TriggeredAbilityDef { kind: TriggerKind::EntersBattlefield, ... }]` and `nevermore.rs:43` implements `has_etb_handler() -> true`. The engine dispatches this as a `PendingTrigger::EnteredBattlefield` at `triggers.rs:534`, which goes on the stack and resolves via `resolve_next_trigger` at `triggers.rs:1243-1249`.

**Description:**
"As this enchantment enters" is a replacement effect (CR 614.1c), not a triggered ability. The choice must happen as part of the entering process, before anyone receives priority. The current implementation uses an ETB trigger that goes on the stack, creating a priority window between Nevermore entering the battlefield and the name being chosen. During this window, players can cast spells and activate abilities — which violates the ruling: "No one can cast spells or activate abilities between the time a card is named and the time that Nevermore's ability starts to work." An opponent could cast the very spell the controller intends to name while the ETB trigger is on the stack but unresolved. The correct implementation would use an as-enters replacement effect (like `entering_modifier_zones` / `modify_creature_entering_counters` pattern) that resolves the name choice during the `move_object` call, before any events fire.

**Engine path:**
- nevermore.rs:33-38 (ETB trigger declaration)
- nevermore.rs:43-68 (ETB handler with AwaitingAction)
- triggers.rs:520-547 (EnteredBattlefield event → PendingTrigger creation)
- triggers.rs:1243-1249 (trigger resolution calls on_enter_battlefield)

**Required check:** 8b

**Affected cards:**
- Nevermore
- Any future card with "As [this permanent] enters" wording that uses the ETB trigger path

