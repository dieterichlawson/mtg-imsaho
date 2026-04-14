---
id: back_from_the_brink-04
status: new
card: Back from the Brink
card_file: mtg-engine/src/cards/isd/back_from_the_brink.rs
created: 2026-04-14T21:24:13Z
audit_run_id: 2026-04-14-back_from_the_brink-audit
audit_model: opus
audit_tokens: 20377
audit_duration: 499
---

## Audit Finding

**Oracle text:**
> If you exile a double-faced creature card this way, you'll pay the mana cost of the front face. The token will be a copy of the front face and it won't be able to transform. (Ruling, 2011-09-22)

**Code:**
> state.rs:524: `obj.card_id = card_id;` — the token copy receives the same card_id as the source DFC. helpers.rs:262-293: `apply_transform` checks `o.zone == Zone::Battlefield` but does NOT check `o.is_token`, so token copies with a DFC's card_id will transform when the DFC's transform trigger fires.

**Description:**
Per CR 712.12, a token that's a copy of a transforming double-faced card can't transform — it enters as a copy of the front face only. When Back from the Brink creates a token copy of a DFC creature (e.g., Mayor of Avabruck, Delver of Secrets), `create_token_copy` gives the token the same `card_id` as the original DFC. This means the token's `CardBehavior` includes all transform logic defined by the DFC. Since `apply_transform` (helpers.rs:262) has no `is_token` guard, the token will transform when the DFC's transform conditions are met (e.g., upkeep check for werewolves). The token would incorrectly gain the back face's name, keywords, subtypes, and power/toughness. The fix should either add an `is_token` guard to `apply_transform` or strip the DFC behavior from token copies.

**Engine path:**
- state.rs:479-527 (create_token_copy — sets card_id to source DFC's card_id)
- helpers.rs:262-293 (apply_transform — no is_token check)

**Required check:** 8g

**Affected cards:**
- Back from the Brink (when targeting any DFC creature)
- Any card that creates token copies of DFCs via create_token_copy (e.g., Cackling Counterpart, Rite of Replication if implemented)

