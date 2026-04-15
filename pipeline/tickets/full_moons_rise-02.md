---
id: full_moons_rise-02
status: new
card: Full Moon's Rise
card_file: mtg-engine/src/cards/isd/full_moons_rise.rs
created: 2026-04-15T03:51:13Z
audit_run_id: 2026-04-14-full_moons_rise-audit
audit_model: opus
audit_tokens: 21104
audit_duration: 485
---

## Audit Finding

**Oracle text:**
> Sacrifice this enchantment: Regenerate all Werewolf creatures you control.

**Code:**
> full_moons_rise.rs:74-82: The subtype check uses `registry.card_data(o.card_id).subtypes` (always front-face data) combined with `o.subtypes`, without checking `back_face_data()` for transformed DFCs.

**Description:**
The `on_activate_ability` method identifies Werewolf creatures using a manual subtype check that reads `registry.card_data(o.card_id).subtypes` — which always returns front-face subtypes regardless of transform state — and unions them with `o.subtypes`. Per CR 712.8d-e, a transformed DFC on the battlefield has only its back face's characteristics. The continuous effect on this same card correctly uses `CreatureFilter::HasSubtype("Werewolf")` (state.rs:831-849), which checks back-face subtypes when `is_transformed` is true. This creates a divergence: the continuous effect (+1/+0, trample) and the activated ability (regeneration) use different subtype-checking logic for the same set of creatures. For a hypothetical transformed DFC with "Werewolf" only on the back face (not the front), the continuous effect would correctly identify it as a Werewolf, but the activated ability would miss it. All current ISD werewolves have "Werewolf" on both faces, so this does not cause incorrect behavior today, but the code path violates CR 712.8d-e.

**Engine path:**
- full_moons_rise.rs:74-82 (manual subtype check in on_activate_ability)
- state.rs:831-849 (correct HasSubtype filter with DFC handling)

**Required check:** 8d

**Affected cards:**
- Full Moon's Rise
- Any card whose `on_activate_ability` manually checks subtypes without DFC awareness

## Tests

### regenerate_transformed_werewolf
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player controls Full Moon's Rise and a transformed DFC Werewolf (e.g., Gatstaf Howler, back face of Gatstaf Shepherd). Player activates the sacrifice ability. Verify the transformed Werewolf receives a regeneration shield — confirming the subtype check works for transformed DFCs.

