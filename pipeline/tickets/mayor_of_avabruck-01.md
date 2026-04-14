---
id: mayor_of_avabruck-01
status: new
card: Mayor of Avabruck
card_file: mtg-engine/src/cards/isd/mayor_of_avabruck.rs
created: 2026-04-14T20:57:26Z
audit_run_id: 2026-04-14-mayor_of_avabruck-audit
audit_model: opus
audit_tokens: 15497
audit_duration: 426
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) "While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face."

**Code:**
> state.rs `move_object` cleanup block (lines ~572-583): clears `is_transformed = false` but does NOT clear `obj.name`. After `apply_transform` sets `obj.name = "Howlpack Alpha"`, dying leaves the object with `name = "Howlpack Alpha"` and `is_transformed = false` in the graveyard.

**Description:**
When Howlpack Alpha (the transformed back face) leaves the battlefield, `move_object` resets `is_transformed` to false but does not restore `obj.name` to the front-face name "Mayor of Avabruck". The `obj_name()` accessor reads `obj.name` directly with no registry fallback. This means the card is identified as "Howlpack Alpha" in the graveyard, exile, hand, or library — violating CR 712.8a. Any effect that searches for "Mayor of Avabruck" by name in those zones (e.g., reanimation, tutor) would fail to find it. If later returned to the battlefield, it would enter with the stale back-face name despite `is_transformed` being false.

**Engine path:**
- state.rs: `move_object` cleanup block (~line 580 clears `is_transformed`, ~line 583 does not clear `name`)
- cards/helpers.rs:285-289 — `apply_transform` sets `obj.name` to back-face name

**Required check:** 8a

**Affected cards:**
- Mayor of Avabruck // Howlpack Alpha
- All double-faced cards using `helpers::apply_transform` (known engine-wide issue per auditor-insights.md)

