---
id: kruin_outlaw-02
status: closed-duplicate
card: Kruin Outlaw
card_file: mtg-engine/src/cards/isd/kruin_outlaw.rs
created: 2026-04-14T20:54:55Z
audit_run_id: 2026-04-14-kruin_outlaw-audit
audit_model: opus
audit_tokens: 13110
audit_duration: 275
duplicate_of: merged-dfc-zone-cleanup-02
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face.

**Code:**
> In `state.rs:564-583`, `move_object` clears `is_transformed` (line 580) but does not reset `obj.name`, `obj.keywords`, or `obj.subtypes` to front-face values. In `helpers.rs:286-289`, `apply_transform` writes back-face values to these fields: `obj.name = "Terror of Kruin Pass"`, `obj.keywords = [DoubleStrike]`, `obj.subtypes = ["Werewolf"]`. In `state.rs:746-748`, `obj_name()` reads `obj.name` directly with no registry fallback.

**Description:**
When Terror of Kruin Pass (back face) leaves the battlefield (dies, is exiled, bounced), `move_object` resets `is_transformed` to false but leaves `obj.name` as "Terror of Kruin Pass". The `obj_name()` function reads this stale value directly, so the card appears with its back-face name in the graveyard, exile, or hand. Per CR 712.8a, in any zone other than the battlefield or stack, the card should have only front-face characteristics — name "Kruin Outlaw". While `has_keyword` and `HasSubtype` have registry-based fallbacks that key off `is_transformed` (masking stale keyword/subtype values), `obj_name()` has no such fallback, making the name field the most impactful stale characteristic. This could cause incorrect behavior for cards that reference card names in the graveyard (e.g., "return a card named X from your graveyard").

**Engine path:**
- state.rs:572-583 (zone-change cleanup block — clears is_transformed but not name/keywords/subtypes)
- helpers.rs:283-292 (apply_transform writes back-face name/keywords/subtypes to obj)
- state.rs:746-748 (obj_name reads obj.name with no registry fallback)

**Required check:** 8a

**Affected cards:**
- Kruin Outlaw / Terror of Kruin Pass
- All DFCs that use `helpers::apply_transform` (all ISD werewolves, Bloodline Keeper, etc.)
