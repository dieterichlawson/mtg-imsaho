## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control an Island or a Mountain.
{T}: Add {U} or {R}.
**Type line**: Land
**Status**: ISSUE

### Code issues

- `controller_has_matching_land` only checks `o.subtypes` (runtime object field) but never consults the registry, so it fails to detect basic Island and Mountain cards on the battlefield. The function signature is `fn controller_has_matching_land(state: &GameState, object_id: ObjectId) -> bool` — no registry parameter. For regular (non-token) cards, subtypes are stored in `CardData.subtypes` and accessed via the registry, not in `GameObject.subtypes`. `setup_game` in `engine.rs` copies `keywords` and `card_types` to the object but never copies `subtypes`; `create_object` initialises `subtypes: Vec::new()`. Therefore `o.subtypes` is always empty for a basic Island or Mountain, and the check returns `false` regardless of what the player controls. Sulfur Falls always enters tapped even when the player controls an Island or Mountain.
  - Oracle text says: `"This land enters tapped unless you control an Island or a Mountain."`
  - Code does: `o.subtypes.iter().any(|s| s == "Island") || o.subtypes.iter().any(|s| s == "Mountain")` — `o.subtypes` is `Vec::new()` for all regular card objects (`mtg-engine/src/cards/isd/sulfur_falls.rs` lines 19–23; `mtg-engine/src/state.rs` line 272 `subtypes: Vec::new()`; `mtg-engine/src/engine.rs` lines 2678–2682 where `obj.subtypes` is not set from `card_data.subtypes`).
  - Contrast with the correct dual-check pattern used in `state.rs` `check_condition` (lines 1085–1092): `o.subtypes.iter().any(|s| s == subtype) || registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == subtype)).unwrap_or(false)`.

### Tricky interactions checked

- **Enters tapped with no matching lands**: Correctly enters tapped (returns `true` for tapped); base case works.
- **Enters untapped when player controls a basic Island**: FAIL — `o.subtypes` is empty for basic Island (subtype only in `card_data().subtypes` via registry); the check returns `false`, land incorrectly enters tapped.
- **Enters untapped when player controls a basic Mountain**: FAIL — same root cause as Island case.
- **Enters untapped when player controls another Sulfur Falls**: FAIL — Sulfur Falls itself has no subtypes (`subtypes: vec![]` in its `card_data`), so this is correctly handled as "no match" regardless. Not a separate bug.
- **Enters untapped when player controls a non-basic land with Island subtype (e.g., Tropical Island)**: Depends on whether that card's subtype is in `o.subtypes` or only in `card_data().subtypes`. Since `setup_game` never copies subtypes to the object, this would also fail for any regular (non-token) dual land.
- **Self-exclusion (`o.id != object_id`)**: Correctly excluded — Sulfur Falls cannot use itself to qualify.
- **Mana ability zone/tapped check**: Correctly returns empty `vec![]` when either not on battlefield or already tapped (`mtg-engine/src/cards/isd/sulfur_falls.rs` lines 57–74).
- **Mana produced ({U} and {R})**: Correctly produces `ManaType::Blue` and `ManaType::Red` matching oracle text `{T}: Add {U} or {R}`.
- **Card types and cost**: Correctly `CardType::Land`, no mana cost (`cost: None`), no supertypes.
- **Registry is passed but ignored in `on_enter_battlefield`**: Parameter named `_registry` at `mtg-engine/src/cards/isd/sulfur_falls.rs` line 43 — registry available but not forwarded to `controller_has_matching_land`, which is the root cause of the subtype bug.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- **Enters tapped when controller has no matching lands**: NOT TESTED
- **Enters untapped when controller has a basic Island**: NOT TESTED
- **Enters untapped when controller has a basic Mountain**: NOT TESTED
- **Mana ability produces {U}**: NOT TESTED
- **Mana ability produces {R}**: NOT TESTED
- **Card registered as Land type**: `mtg-engine/tests/innistrad_simple_cards.rs:107–112` (only checks `CardType::Land` membership)
- **Mana ability unavailable while tapped**: NOT TESTED
- **Mana ability unavailable outside battlefield**: NOT TESTED
