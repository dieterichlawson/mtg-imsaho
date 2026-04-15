---
id: moonmist-01
status: new
card: Moonmist
card_file: mtg-engine/src/cards/isd/moonmist.rs
created: 2026-04-15T03:54:42Z
audit_run_id: 2026-04-14-moonmist-audit
audit_model: opus
audit_tokens: 24806
audit_duration: 535
---

## Audit Finding

**Oracle text:**
> Transform all Humans.

**Code:**
> moonmist.rs:83-88:
> ```rust
> if let Some(back) = behavior.back_face_data() {
>     if let Some(obj) = state.get_object_mut(*hid) {
>         obj.name.clone_from(&back.name);
>         if let Some(p) = back.power {
>             obj.power = Some(p);
>         }
>         if let Some(t) = back.toughness {
>             obj.toughness = Some(t);
>         }
>         obj.keywords.clone_from(&back.keywords);
>         obj.subtypes.clone_from(&back.subtypes);
>     }
> }
> ```

**Description:**
Moonmist manually transforms Humans by directly mutating obj.power and obj.toughness to the target face's values. Every other DFC card in the codebase uses `helpers::apply_transform()` (helpers.rs:262), which correctly avoids touching P/T — each card instead provides a `dynamic_pt()` method that returns the correct P/T based on `is_transformed`. Moonmist's redundant P/T mutation creates stale values that persist through zone changes: `move_object` clears `is_transformed` but does NOT clear `obj.power`/`obj.toughness`. After a transformed creature is bounced and recast, `dynamic_pt()` returns `None` (not transformed), so `effective_power` (state.rs:1057) falls through to the stale `obj.power` — showing back-face P/T on the front face. The stale value is never corrected: `apply_transform`'s front-face restore also doesn't touch P/T, so even subsequent upkeep-triggered transforms back to front face don't fix the contaminated value. Example: Moonmist transforms Gatstaf Shepherd (2/2) to Gatstaf Howler (3/3), setting obj.power=3. After bounce and recast, Gatstaf Shepherd appears as 3/3 instead of 2/2, permanently.

**Engine path:**
- mtg-engine/src/cards/isd/moonmist.rs:83-88 (back-face P/T mutation)
- mtg-engine/src/cards/isd/moonmist.rs:74-75 (front-face P/T mutation, also redundant)
- mtg-engine/src/state.rs:1057 (effective_power falls through to stale obj.power when dynamic_pt returns None)
- mtg-engine/src/state.rs:1118 (effective_toughness same pattern)
- mtg-engine/src/state.rs:572-583 (move_object cleanup — clears is_transformed but not power/toughness)
- mtg-engine/src/cards/helpers.rs:275-292 (apply_transform correctly avoids P/T mutation)

**Required check:** 8a

**Affected cards:**
- Moonmist (only card that manually mutates obj.power/obj.toughness during transform instead of using apply_transform)

## Tests

### moonmist_transform_stale_pt_after_bounce
Source ticket: (new)
Implementation: (not yet written)
Scenario: Put Gatstaf Shepherd (2/2 front / 3/3 back) on the battlefield under P0. Cast and resolve Moonmist to transform it. Verify is_transformed=true and effective_power=3. Then move Gatstaf Shepherd to hand (simulating bounce). Verify is_transformed=false. Then move it back to the battlefield. Assert effective_power(id, registry) == 2 and effective_toughness(id, registry) == 2. Currently fails: effective_power returns 3 due to stale obj.power.

