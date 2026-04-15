---
id: grimoire_of_the_dead-01
status: closed-duplicate
card: Grimoire of the Dead
card_file: mtg-engine/src/cards/isd/grimoire_of_the_dead.rs
created: 2026-04-14T20:57:12Z
audit_run_id: 2026-04-14-grimoire_of_the_dead-audit
audit_model: opus
audit_tokens: 16027
audit_duration: 412
duplicate_of: merged-activation-cost-variants-01
---

## Audit Finding

**Oracle text:**
> {T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control.

**Code:**
> `activated_abilities()` at grimoire_of_the_dead.rs:68 gates on `study_counters >= 3` but no code explicitly removes three study counters. The `ActivatedAbilityDef` (grimoire_of_the_dead.rs:69-78) has no counter-removal cost field. At engine.rs:2662-2663, `SacrificeCost::SacrificeThis` calls `sacrifice()`, which calls `move_object()` to graveyard, which clears ALL counters at state.rs:578 (`obj.counters.clear()`).

**Description:**
Per CR 602.2b/601.2h, "Remove three study counters" is an activation cost that must be explicitly paid. The engine's `ActivatedAbilityDef` has no field for counter-removal costs, so the card uses a gating check (>= 3 counters) instead of actual cost payment. The counters are never removed as a discrete game action — they vanish when the sacrifice clears all counters via zone-change cleanup. This means: (1) no "counter removed" event fires, so abilities that trigger on counter removal would miss it; (2) if the Grimoire has more than 3 study counters, all are cleared instead of exactly 3; (3) the counter removal and sacrifice are not independent cost actions — per CR 118.11, each stated cost action must actually occur.

**Engine path:**
- grimoire_of_the_dead.rs:68 (gating check only)
- engine.rs:2662-2663 (sacrifice handles zone move)
- state.rs:578 (counters.clear() during zone-change cleanup)

**Required check:** 8i

**Affected cards:**
- Grimoire of the Dead
- Any card with "Remove N counters" as an activation cost (engine-wide: ActivatedAbilityDef lacks counter-removal cost support)
