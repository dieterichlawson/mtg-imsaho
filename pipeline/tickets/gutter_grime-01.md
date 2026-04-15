---
id: gutter_grime-01
status: closed-duplicate
card: Gutter Grime
card_file: mtg-engine/src/cards/isd/gutter_grime.rs
created: 2026-04-14T21:27:53Z
audit_run_id: 2026-04-14-gutter_grime-audit
audit_model: opus
audit_tokens: 18563
audit_duration: 350
duplicate_of: merged-trigger-source-zone-gate-01
---

## Audit Finding

**Oracle text:**
> Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."

**Code:**
> gutter_grime.rs:46-49:
> ```rust
> let controller = match state.get_object(self_id) {
>     Some(o) if o.zone == Zone::Battlefield => o.controller,
>     _ => return,
> };
> ```

**Description:**
The handler returns early when Gutter Grime is not on the battlefield at trigger resolution time, skipping both the counter placement AND the token creation. Per CR 608.2d, a resolving ability performs "as much of each applicable instruction as possible." If Gutter Grime left the battlefield between trigger creation and resolution (e.g., opponent casts Disenchant in response to the death trigger), the counter placement correctly cannot happen (not a permanent), but the "then create a green Ooze creature token" instruction IS still possible and should execute. The token's P/T would be 0 (source has no visible slime counters) and it would die to SBA, but the trigger should still partially resolve. This also re-checks the "you control" condition at resolution (`dead_controller != controller`), which is incorrect — "Whenever a nontoken creature you control dies" has no intervening-if clause, so the condition is only checked when the trigger event occurs, not at resolution (CR 603.4).

**Engine path:**
- gutter_grime.rs:46-49 (zone check returns early)
- gutter_grime.rs:51-52 (controller re-check at resolution)

**Required check:** 8b (trigger resolution behavior)

**Affected cards:**
- Gutter Grime
- Potentially any card with `on_any_creature_dies` that gates on `zone == Battlefield` at resolution and performs multiple sequential actions where later actions don't depend on the source being present
