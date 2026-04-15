---
id: hanweir_watchkeep-01
status: deduped
card: Hanweir Watchkeep
card_file: mtg-engine/src/cards/isd/hanweir_watchkeep.rs
created: 2026-04-14T21:30:20Z
audit_run_id: 2026-04-14-hanweir_watchkeep-audit
audit_model: opus
audit_tokens: 18615
audit_duration: 468
deduped_into: merged-dfc-zone-cleanup-01
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) "While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face."

**Code:**
> `move_object` (state.rs:572-582) clears `is_transformed = false` but does not reset `obj.name`. `obj_name()` (state.rs:746-748): `let name = self.get_object(id).map_or_else(|| "?".into(), |o| o.name.clone());` — reads `obj.name` directly with no registry fallback.

**Description:**
When Bane of Hanweir (the transformed back face) leaves the battlefield — via destruction, exile, bounce, or any other zone change — `move_object` clears `is_transformed` to false but does not restore `obj.name` to the front-face value "Hanweir Watchkeep". The card retains its back-face name "Bane of Hanweir" in the graveyard, exile, hand, or library. Unlike keywords and subtypes, which have registry-based fallbacks in `has_keyword` that read from `card_data()` when `is_transformed` is false, `obj_name()` reads `obj.name` directly. Any game effect that references the card by name in a non-battlefield zone (e.g., Regrowth naming a card, graveyard recursion, or name-based search) will see the wrong name. This is an engine-wide DFC issue affecting all double-faced cards that transform via `helpers::apply_transform`.

**Engine path:**
- state.rs:572-582 (`move_object` cleanup block)
- state.rs:746-748 (`obj_name` — no registry fallback for name)
- cards/helpers.rs:262-293 (`apply_transform` sets `obj.name` to back-face name)

**Required check:** 8a

**Affected cards:**
- Hanweir Watchkeep // Bane of Hanweir
- All DFCs that transform via `helpers::apply_transform`
