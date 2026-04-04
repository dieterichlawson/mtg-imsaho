## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Type line**: Land
**Status**: ISSUE

### Code issues
- Subtype checking in `controller_has_matching_land` function (lines 21-22) only checks runtime object subtypes, not registry data
  - Oracle text says: `This land enters tapped unless you control a Mountain or a Plains.`
  - Code does: Only checks `o.subtypes.iter().any(|s| s == "Mountain")` and `o.subtypes.iter().any(|s| s == "Plains")` but doesn't check `registry.card_data()` subtypes. This could miss lands that have Mountain/Plains subtypes in registry but not in runtime object (compare with `check_condition` in `state.rs` which correctly checks both sources).

### Tricky interactions checked
- **Simultaneous ETB with other lands**: PASS - Code correctly excludes self (`o.id != object_id`) and existing tests show this works correctly
- **"you control" scoping**: PASS - Code correctly gets the controller of the entering Clifftop Retreat and checks their battlefield
- **"unless" replacement effect timing**: PASS - Implemented in `on_enter_battlefield` which is the correct timing for ETB replacement effects
- **Mana ability choice between red and white**: PASS - Two separate `ManaAbilityDef` entries allow player to choose between colors
- **Basic land types vs basic lands distinction**: PASS - Code correctly checks for land subtypes (Mountain/Plains) rather than basic supertype

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card enters tapped without matching lands: `mtg-engine/tests/innistrad_simple_cards.rs:31`
- Card enters untapped with Mountain present: `mtg-engine/tests/innistrad_simple_cards.rs:51` 
- Mana abilities produce red or white: `mtg-engine/tests/innistrad_simple_cards.rs:76`
- Subtype checking with both registry and runtime data: NOT TESTED
- Simultaneous ETB with other typed lands: NOT TESTED
- Interaction with nonbasic lands that have basic land types: NOT TESTED