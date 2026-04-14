---
id: instigator_gang-03
status: new
card: Instigator Gang
card_file: mtg-engine/src/cards/isd/instigator_gang.rs
created: 2026-04-14T21:28:14Z
audit_run_id: 2026-04-14-instigator_gang-audit
audit_model: opus
audit_tokens: 13518
audit_duration: 264
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face.

**Code:**
> `state.rs:572-583` — `move_object` cleanup block clears `is_transformed` (line 580) but does not clear `obj.name`. `helpers.rs:287` — `apply_transform` sets `obj.name` to the back-face name when transforming.

**Description:**
When Wildblood Pack (the transformed back face) leaves the battlefield, `move_object` resets `is_transformed` to false but does not reset `obj.name`. Since `apply_transform` (helpers.rs:287) wrote "Wildblood Pack" into `obj.name` during the transform, that name persists after the zone change. The engine reads `obj.name` directly with no registry fallback (unlike keywords/subtypes which have `is_transformed`-aware registry lookups). The card therefore retains the name "Wildblood Pack" in the graveyard, hand, or exile, violating CR 712.8a. This is a known engine-wide DFC issue documented in `pipeline/prompts/auditor-insights.md`.

**Engine path:**
- `state.rs:572-583` (move_object cleanup — no name reset)
- `helpers.rs:262-287` (apply_transform writes back-face name)

**Required check:** 8a

**Affected cards:**
- Instigator Gang / Wildblood Pack
- All DFCs that use `helpers::apply_transform`

