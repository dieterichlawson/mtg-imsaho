---
id: army_of_the_damned-01
status: new
card: Army of the Damned
card_file: mtg-engine/src/cards/isd/army_of_the_damned.rs
created: 2026-04-14T20:46:53Z
audit_run_id: 2026-04-14-army_of_the_damned-audit
audit_model: opus
audit_tokens: 21975
audit_duration: 465
---

## Audit Finding

**Oracle text:**
> Create thirteen tapped 2/2 black Zombie creature tokens.

**Code:**
> `create_token_internal` at state.rs:429 creates tokens with `tapped: false`. The `EnteredBattlefield` event is emitted at state.rs:470-473 while the token is untapped. Control then returns to `on_resolve` at army_of_the_damned.rs:53-55, which sets `obj.tapped = true` on each token after creation.

**Description:**
Per CR 701.6, "Create thirteen tapped tokens" means the tokens enter the battlefield already in a tapped state — tapped is part of the token's creation specification, not a post-entry modification. The implementation creates tokens untapped, emits the EnteredBattlefield event, and then taps them. This exposes a brief intermediate state where tokens exist on the battlefield untapped. While the event does not encode tapped status and trigger handlers read current state at processing time (making the behavior functionally correct in the current engine), any replacement effect that inspects entering-tapped status (running inside `apply_entering_copy_replacement` at state.rs:461, before the tapping) would see the wrong state. The `create_token_with_subtypes` API lacks a `tapped` parameter, making this an engine limitation rather than a card-specific bug. All other ISD cards creating tapped tokens (Geist of Saint Traft, Kessig Cagebreakers, Grimgrin) use the same post-creation tapping pattern.

**Engine path:**
- state.rs:429 (`tapped: false` in `create_token_internal`)
- state.rs:461 (`apply_entering_copy_replacement` runs with token untapped)
- state.rs:470-473 (`EnteredBattlefield` event emitted with token untapped)
- army_of_the_damned.rs:53-55 (`obj.tapped = true` set after creation)

**Required check:** 8g

**Affected cards:**
- Army of the Damned
- Geist of Saint Traft (geist_of_saint_traft.rs:75)
- Kessig Cagebreakers (kessig_cagebreakers.rs:73)
- Grimgrin, Corpse-Born (grimgrin_corpse_born.rs:54)
- Diregraf Ghoul (diregraf_ghoul.rs:32)

