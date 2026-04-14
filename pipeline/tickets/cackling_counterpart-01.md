---
id: cackling_counterpart-01
status: new
card: Cackling Counterpart
card_file: mtg-engine/src/cards/isd/cackling_counterpart.rs
created: 2026-04-14T21:25:14Z
audit_run_id: 2026-04-14-cackling_counterpart-audit
audit_model: opus
audit_tokens: 27785
audit_duration: 560
---

## Audit Finding

**Oracle text:**
> Create a token that's a copy of target creature you control.

**Code:**
> `state.rs:486-505` — `create_token_copy` reads `(name, power, toughness, card_id, is_legendary)` from the runtime game object at line 487, then reads `(colors, keywords, card_types, subtypes)` from `registry.card_data(card_id)` at line 490. These two data sources can disagree.

**Description:**
`create_token_copy` uses an inconsistent mix of data sources for the token's characteristics. Per CR 707.2, a copy effect should copy the "copiable values" — a consistent set of characteristics from one source. The current code splits reads across the runtime object and the card registry, causing failures in two scenarios:

(a) **Copying a generic token** (e.g., a 2/2 black Zombie created by `create_token_with_subtypes`): The token has `card_id = CardId(0)` (sentinel value). `registry.card_data(CardId(0))` returns `None`, and `.unwrap_or_default()` at line 505 yields empty vectors for keywords, subtypes, card_types, and colors. The token copy loses its "Zombie" subtype, "Creature" card type, and Black color — only name and P/T survive.

(b) **Copying a transformed DFC** (e.g., a werewolf in back-face form): `card_id` still points to the front face. `registry.card_data(card_id)` returns front-face keywords, subtypes, card_types, and colors-from-mana-cost. But the object has the back-face name (updated by `apply_transform` at helpers.rs:287). The token gets a contradictory mix: back-face name with front-face creature types, keywords, subtypes, and colors.

**Engine path:**
- state.rs:486-505 (`create_token_copy` — inconsistent data sources)
- state.rs:409-475 (`create_token_internal` — sets fields from parameters)
- cards/helpers.rs:262-293 (`apply_transform` — updates obj.name/keywords/subtypes but not power/toughness/card_types)

**Required check:** 8g

**Affected cards:**
- Cackling Counterpart
- Any card that calls `create_token_copy` on a generic token or transformed DFC

