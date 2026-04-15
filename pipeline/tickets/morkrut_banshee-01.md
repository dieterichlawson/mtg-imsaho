---
id: morkrut_banshee-01
status: closed-duplicate
card: Morkrut Banshee
card_file: mtg-engine/src/cards/isd/morkrut_banshee.rs
created: 2026-04-14T21:54:58Z
audit_run_id: 2026-04-14-morkrut_banshee-audit
audit_model: opus
audit_tokens: 16831
audit_duration: 1625
duplicate_of: merged-trigger-target-recheck-01
---

## Audit Finding

**Oracle text:**
> Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.

**Code:**
> `triggers.rs:1243-1249`: `PendingTrigger::EnteredBattlefield { object_id, card_id, chosen_targets, .. } => { if let Some(behavior) = registry.get(card_id) { behavior.on_enter_battlefield(state, object_id, &chosen_targets, registry); } }` — no target legality re-check before calling handler.

**Description:**
Per CR 608.2b, when a triggered ability with targets tries to resolve, all targets must still be legal. If all targets are illegal, the ability is removed from the stack and does nothing. The `resolve_next_trigger` function dispatches ETB triggers directly to `on_enter_battlefield` without re-checking target legality. If the creature targeted by Morkrut Banshee's trigger becomes illegal between the trigger going on the stack and resolving (e.g., leaves the battlefield, gains hexproof/protection), the trigger resolves anyway, pushing a stale `TemporaryEffect::ModifyPT` entry. While this has no visible effect when the target is gone, it violates CR 608.2b and leaves a stale entry that could interact incorrectly if the target returns to the battlefield in the same turn (compounding with Finding 2).

**Engine path:**
- triggers.rs:1232 (`resolve_next_trigger`)
- triggers.rs:1243-1249 (EnteredBattlefield dispatch, no target re-check)

**Required check:** 8b (trigger dispatch), documented in auditor-insights.md ("Triggered ability resolution skips target legality check")

**Affected cards:**
- Morkrut Banshee
- All cards with targeted triggered abilities (engine-wide)
