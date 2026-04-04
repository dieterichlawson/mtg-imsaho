## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Swamp or a Forest.
{T}: Add {B} or {G}.
**Type line**: Land
**Status**: ISSUE

### Code issues

- Subtype detection in `controller_has_matching_land` only checks `obj.subtypes` (object-level field), which is always empty for non-token regular cards — causing Woodland Cemetery to always enter tapped even when the controller controls a Swamp or Forest.
  - Oracle text says: `"This land enters tapped unless you control a Swamp or a Forest."`
  - Code does: `o.subtypes.iter().any(|s| s == "Swamp") || o.subtypes.iter().any(|s| s == "Forest")` (`mtg-engine/src/cards/isd/woodland_cemetery.rs`, lines 19–22). For all non-token cards (including basic lands Swamp and Forest), `obj.subtypes` is initialized to `Vec::new()` in `create_object` (`state.rs:272`) and never populated from `CardData.subtypes`. The `setup_game` function copies `colors`, `name`, `keywords`, and `card_types` from the registry to each card object but does NOT copy `subtypes` (`engine.rs` around line 2680). Therefore this check always returns `false` for actual Swamp/Forest permanents, and Woodland Cemetery always enters the battlefield tapped regardless of what lands the controller has.

### Tricky interactions checked

- **Subtype detection: obj.subtypes vs registry.card_data().subtypes**: FAIL. `controller_has_matching_land` only checks `o.subtypes` (object-level). For non-token cards, the engine never copies `CardData.subtypes` to `obj.subtypes`. Compared to `state.rs`'s `matches_filter` (lines 665–672), which correctly checks `registry.card_data(creature.card_id).subtypes` first, then falls back to `obj.subtypes`, this function is incomplete and always returns false for regular Swamp/Forest lands.
- **Lands played from hand entering tapped correctly when no matching land**: PASS. When no Swamp or Forest is present, `controller_has_matching_land` returns `false` (for the correct reason — `obj.subtypes` is empty), `obj.tapped` is set to `true`, and the log message fires. This branch works.
- **ETB trigger fires at the right time**: PASS. `on_enter_battlefield` is dispatched via the trigger system in `triggers.rs` (line 897) after `move_object` puts the land on the battlefield; the land's `object_id` is present in the battlefield zone when the check runs.
- **Self-exclusion in the land scan**: PASS. The predicate correctly uses `o.id != object_id` (line 18) to exclude Woodland Cemetery itself from the check, which is correct since it carries no Swamp/Forest subtypes.
- **Mana ability produces {B} or {G}**: PASS. Two `ManaAbilityDef` entries are returned, one producing `(ManaType::Black, 1)` and one producing `(ManaType::Green, 1)`, both with `requires_tap: true`. This correctly models `{T}: Add {B} or {G}`.
- **Mana ability gated on untapped**: PASS. `mana_abilities` returns an empty vec when `obj.tapped` is true, so a tapped Woodland Cemetery produces no mana.
- **Oracle text field content**: PASS. The `oracle_text` field in `card_data()` reads `"This land enters tapped unless you control a Swamp or a Forest.\n{T}: Add {B} or {G}."` which matches the Scryfall oracle text exactly.
- **Type line (Land, no subtypes)**: PASS. `card_types: vec![CardType::Land]`, `subtypes: vec![]`, `supertypes: vec![]`. Woodland Cemetery is a nonbasic land with no land subtype; this is correct.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Enters tapped when no matching land present: `mtg-engine/tests/innistrad_simple_cards.rs` (clifftop_retreat test at line 31 covers the analogous case; no dedicated test for Woodland Cemetery in the no-Swamp/no-Forest path)
- Enters untapped when controller has a Swamp: `mtg-engine/tests/innistrad_simple_cards.rs:123` (`woodland_cemetery_enters_untapped_with_swamp`) — BUT this test manually patches `state.get_object_mut(swp).unwrap().subtypes = vec!["Swamp".into()]` on the created Swamp object. In a real game the engine never populates `obj.subtypes` for non-token cards, so the test bypasses rather than catches the underlying subtype-detection bug. The test passes while masking the issue.
- Enters untapped when controller has a Forest: NOT TESTED
- Mana ability produces {B}: NOT TESTED directly for Woodland Cemetery (only card_data tested at line 115)
- Mana ability produces {G}: NOT TESTED directly for Woodland Cemetery
- Mana ability unavailable when tapped: NOT TESTED
- Self-exclusion (Cemetery doesn't count itself as a Swamp/Forest): NOT TESTED
