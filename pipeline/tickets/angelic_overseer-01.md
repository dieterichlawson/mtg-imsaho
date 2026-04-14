---
id: angelic_overseer-01
status: new
card: Angelic Overseer
card_file: mtg-engine/src/cards/isd/angelic_overseer.rs
created: 2026-04-14T21:20:38Z
audit_run_id: 2026-04-14-angelic_overseer-audit
audit_model: opus
audit_tokens: 13687
audit_duration: 284
---

## Audit Finding

**Oracle text:**
> As long as you control a Human, this creature has hexproof and indestructible.

**Code:**
> engine.rs:3790-3792 — `KeepOneDestroyRest` handler iterates all creatures and calls `try_destroy` sequentially:
> ```rust
> for cid in all_creatures {
>     if !kept.contains(&cid) {
>         crate::destruction::try_destroy(state, cid, registry);
>     }
> }
> ```
> Same pattern in divine_reckoning.rs:78-81 (inline path for 0-1 creature players).

**Description:**
Spell-based mass destruction effects (e.g., Divine Reckoning's "Destroy the rest") call `try_destroy` sequentially without snapshotting indestructible status first. Each `try_destroy` call immediately moves the destroyed creature to the graveyard via `destroy() -> move_object()`. If HashMap iteration processes a Human before Angelic Overseer, the Human leaves the battlefield first, causing `has_keyword(Indestructible)` to return false for Angelic Overseer when its `try_destroy` is reached. The Angel is then incorrectly destroyed. The SBA code at sba.rs:107-110 already has the correct fix — it snapshots all indestructible creatures before processing any deaths — but the spell-effect mass destruction path lacks an equivalent snapshot. This violates the ruling that Angelic Overseer survives simultaneous destruction with a Human, and more broadly violates CR 608.2c (a single "destroy all" instruction is one simultaneous event, not sequential).

**Engine path:**
- engine.rs:3790-3792 (KeepOneDestroyRest handler)
- divine_reckoning.rs:78-81 (inline destruction loop)
- sba.rs:107-110 (correct snapshot pattern for reference)
- destruction.rs:33-49 (try_destroy immediately moves to graveyard)

**Required check:** 8j (ruling 1 coverage)

**Affected cards:**
- Angelic Overseer (conditional indestructible lost mid-destruction)
- Any creature with conditional indestructible/regeneration that depends on another creature surviving the same mass destruction event
- All cards that use KeepOneDestroyRest or manual try_destroy loops for "destroy all" effects

