## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender\n{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
**Back face oracle text**: Trample
**Type line**: Creature — Lizard Egg (front) / Creature — Lizard Horror (back)
**P/T**: 0/3 (front) / 13/13 (back)
**Status**: ISSUE

### Code issues

- **`card_state` (hatchling counters) not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487
  - Oracle text says: `{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.` (per MTG rule 400.7, counters are lost when a permanent leaves the battlefield and becomes a new object)
  - Code does: `move_object()` clears `obj.counters` (standard counter map) but does NOT clear `obj.card_state`. Ludevic's Test Subject stores hatchling counters in `card_state` under the key `"hatchling_counters"`. When the creature leaves the battlefield (e.g., bounced by Silent Departure) and later re-enters, `card_state` still carries the stale counter value. A card bounced with 4 hatchling counters would transform on the very next activation instead of requiring 5 full activations from scratch.

- **`is_transformed` not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487
  - Oracle text says the front face is Ludevic's Test Subject (0/3 Defender); when a DFC leaves the battlefield it is treated as its front face in all other zones (MTG rule 711.7b), so it must re-enter as the front face.
  - Code does: `move_object()` does not reset `obj.is_transformed`. If Ludevic's Abomination (the transformed state, `is_transformed = true`) is bounced to hand (e.g., by Silent Departure or Lost in the Mist) and then re-cast, the object re-enters the battlefield with `is_transformed = true`, presenting as Ludevic's Abomination (13/13 Trample) instead of Ludevic's Test Subject (0/3 Defender). The activated ability for adding hatchling counters is also never offered, because `activated_abilities()` guards on `!o.is_transformed`. The card is permanently stuck as the back face after any bounce.

### Tricky interactions checked

- **5th activation threshold check**: `new_count = current + 1` is correctly compared with `>= 5`; removing via `card_state.remove("hatchling_counters")` correctly leaves 0 counters regardless of whether the 5th counter was physically stored first. Functionally equivalent to "put it on then remove all". PASS
- **Activated ability unavailable after transform**: `activated_abilities()` guards `Some(o) if o.zone == Zone::Battlefield && !o.is_transformed`; after transform the ability list is empty so the ability cannot be activated again. PASS
- **Defender blocks attacking on front face**: `has_keyword(..., Keyword::Defender)` in `combat.rs:579` reads `obj.keywords`; `apply_transform` correctly sets `obj.keywords = back.keywords` (Trample, no Defender) on transform. After transform the creature is eligible to attack. PASS
- **Trample on back face**: `apply_transform` in `helpers.rs:257–264` sets `obj.keywords = back.keywords = vec![Keyword::Trample]`. `has_keyword` at step 0 checks `obj.keywords.contains(&keyword)`. Back face has Trample. PASS
- **P/T on back face**: `dynamic_pt` returns `Some((13, 13))` when `is_transformed == true`; `effective_power` and `effective_toughness` in `state.rs:868/912` call `behavior.dynamic_pt()` first, so SBA toughness checks and combat damage resolve correctly at 13/13. PASS
- **should_transform returns false**: Ludevic's Test Subject transforms only via its activated ability, not via upkeep conditions. `should_transform` returning `false` prevents erroneous werewolf-style upkeep transformation. PASS
- **Hatchling counter persistence across zone changes** (bounce and recast): FAIL — `card_state` not cleared; see issue 1 above.
- **is_transformed persistence across zone changes** (bounce while transformed): FAIL — `is_transformed` not reset; see issue 2 above.
- **"Then if there are five OR MORE"**: Code uses `if new_count >= 5`, which correctly handles the case where more than 5 counters exist (e.g., if counters were added externally). PASS
- **Cannot activate ability from graveyard/hand**: `activated_abilities()` checks `o.zone == Zone::Battlefield`, so the ability is not offered when the card is not on the battlefield. PASS
- **dynamic_pt used in SBA (zero-toughness check)**: `sba.rs:64–66` calls `state.effective_toughness()` which calls `dynamic_pt`; the 0/3 front face has `power: Some(0)` and `toughness: Some(3)` set in `card_data`, so SBA works correctly on the front face as well. PASS
- **Log message after transform**: After transform, `card_name()` at `engine.rs:1442` reads from `registry.card_data(o.card_id)` which always returns the front-face name "Ludevic's Test Subject". The log at `engine.rs:1806` would therefore say "activated ability on Ludevic's Test Subject" even though the card has just become Ludevic's Abomination. This is a minor log inaccuracy but does not affect gameplay. Not flagged as a separate issue.

### Test coverage

- Basic transform at 5 counters (4 activations no-transform, 5th activates transform, checks name/keywords/subtypes/dynamic_pt): `tier15_cards.rs:1362` — TESTED
- Hatchling counters reset to 0 when card bounces back to hand and is re-cast: NOT TESTED
- `is_transformed` reset to false when Ludevic's Abomination leaves the battlefield: NOT TESTED
- Back face has Trample, no Defender (combat eligibility after transform): NOT TESTED (only keyword presence checked, not actual combat eligibility)
- Ability correctly unavailable after transform: NOT TESTED (tested implicitly by the 5th activation transforming, but no explicit test that a 6th activation is unavailable)
- Correct P/T (13/13) reported by SBA and combat after transform: NOT TESTED
