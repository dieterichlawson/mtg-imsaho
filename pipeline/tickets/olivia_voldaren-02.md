---
id: olivia_voldaren-02
status: deduped
card: Olivia Voldaren
card_file: mtg-engine/src/cards/isd/olivia_voldaren.rs
created: 2026-04-14T20:44:31Z
audit_run_id: 2026-04-14-olivia_voldaren-audit
audit_model: opus
audit_tokens: 17927
audit_duration: 323
deduped_into: merged-zone-cleanup-characteristics-01
---

## Audit Finding

**Oracle text:**
> That creature becomes a Vampire in addition to its other types.

**Code:**
> `olivia_voldaren.rs:113-115`:
> ```rust
> if !obj.subtypes.contains(&"Vampire".to_string()) {
>     obj.subtypes.push("Vampire".to_string());
> }
> ```
> `state.rs:572-583` (zone-change cleanup): clears `tapped`, `summoning_sick`, `damage_marked`, `counters`, `is_transformed`, etc., but NOT `subtypes`.

**Description:**
Olivia's first ability adds "Vampire" to the target creature's runtime `obj.subtypes`. When that creature later leaves the battlefield (dies, is exiled, bounced), `move_object` in state.rs does not clear the `subtypes` field. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The Vampire subtype incorrectly persists through zone changes — if the creature returns to the battlefield (via reanimation, flicker, etc.), it will still be a Vampire without being re-targeted by Olivia. This is a known engine-level gap documented in auditor-insights.md ("Zone-change cleanup does not reset characteristic modifications").

**Engine path:**
- `olivia_voldaren.rs:113-115` (adds subtype)
- `state.rs:572-583` (cleanup block — `subtypes` not cleared)

**Required check:** 8a

**Affected cards:**
- Olivia Voldaren
- Any card that adds subtypes at runtime (same engine gap)
