---
id: bloodline_keeper-01
status: deduped
card: Bloodline Keeper
card_file: mtg-engine/src/cards/isd/bloodline_keeper.rs
created: 2026-04-14T20:44:07Z
audit_run_id: 2026-04-14-bloodline_keeper-audit
audit_model: opus
audit_tokens: 11343
audit_duration: 299
deduped_into: merged-dfc-zone-cleanup-01
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) "While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face."

**Code:**
> state.rs:572–583 — `move_object` cleanup block clears `is_transformed = false` but does not reset `obj.name`. bloodline_keeper.rs:155–157 — transform sets `obj.name = "Lord of Lineage"`. state.rs:746–748 — `obj_name()` reads `obj.name` directly with no registry fallback.

**Description:**
When Bloodline Keeper transforms into Lord of Lineage and then leaves the battlefield (dies, bounced, exiled), `move_object` correctly clears `is_transformed` but leaves `obj.name` as "Lord of Lineage". In the graveyard, hand, or exile, the card retains its back-face name. Per CR 712.8a, a DFC outside the battlefield should have only front-face characteristics — its name should be "Bloodline Keeper". This matters for any effect that references cards by name in non-battlefield zones (e.g., "return a card named Bloodline Keeper from your graveyard" would fail to find it). The engine's keyword and subtype checks have registry-based fallbacks that correctly use `is_transformed` to resolve the right face, but `obj_name()` has no such fallback — it reads `obj.name` directly.

**Engine path:**
- state.rs:572–583 (move_object cleanup)
- state.rs:746–748 (obj_name — no registry fallback)
- bloodline_keeper.rs:155–157 (manual name assignment during transform)

**Required check:** 8a

**Affected cards:**
- Bloodline Keeper // Lord of Lineage
- All other DFCs that set `obj.name` during transform (engine-level bug in `move_object`)
