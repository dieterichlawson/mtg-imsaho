---
id: olivia_voldaren-04
status: new
card: Olivia Voldaren
card_file: mtg-engine/src/cards/isd/olivia_voldaren.rs
created: 2026-04-14T20:44:31Z
audit_run_id: 2026-04-14-olivia_voldaren-audit
audit_model: opus
audit_tokens: 17927
audit_duration: 323
---

## Audit Finding

**Oracle text:**
> {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.

**Code:**
> `olivia_voldaren.rs:133-134`:
> ```rust
> let is_vampire = state.get_object(*target_id)
>     .is_some_and(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string()));
> ```
> Compare with `engine.rs:1971-1974` (target filter):
> ```rust
> TargetFilter::HasSubtype(subtype) => {
>     obj.subtypes.contains(subtype)
>         || registry.card_data(obj.card_id)
>             .is_some_and(|d| d.subtypes.iter().any(|s| s == subtype))
> }
> ```

**Description:**
At resolution time, the steal ability re-checks whether the target is a Vampire by reading only `obj.subtypes`. For regular creature cards, `obj.subtypes` is initialized empty (`state.rs:319`) and is only populated at runtime (e.g., by Olivia's first ability adding "Vampire"). A creature that is naturally a Vampire — with "Vampire" in its registry `card_data().subtypes` but not in the runtime `obj.subtypes` — will pass the targeting filter (which correctly checks both sources) but fail the resolution check. The steal silently does nothing. This means Olivia's second ability cannot steal natural Vampires that haven't first been targeted by her first ability, despite them being legal targets.

**Engine path:**
- `olivia_voldaren.rs:133-134` (resolution check — obj only)
- `engine.rs:1971-1974` (target filter — obj + registry)
- `state.rs:319` (obj.subtypes initialized empty)

**Required check:** 8d

**Affected cards:**
- Olivia Voldaren

