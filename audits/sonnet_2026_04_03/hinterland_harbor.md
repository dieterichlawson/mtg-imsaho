## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
**Type line**: Land
**Status**: ISSUE

### Code issues
- Subtype checking bug in `controller_has_matching_land` function at mtg-engine/src/cards/isd/hinterland_harbor.rs:21-22
  - Oracle text says: `unless you control a Forest or an Island`
  - Code does: Only checks `o.subtypes.iter().any(|s| s == "Forest")` and `o.subtypes.iter().any(|s| s == "Island")`, which only works for tokens. Regular Forest and Island cards store their subtypes in `registry.card_data()`, not in `o.subtypes`. The correct pattern (shown in `state.rs:check_condition`) checks both `o.subtypes` AND `registry.card_data(o.card_id).subtypes`.

### Tricky interactions checked
- Subtype checking for regular vs token lands: FAIL - only checks obj.subtypes, misses regular Forest/Island cards
- Self-exclusion with `o.id != object_id`: PASS - correctly excludes the Harbor itself
- Controller ownership check: PASS - correctly checks lands controlled by the same player
- Zone filtering to Battlefield: PASS - correctly only looks at lands on the battlefield
- Enters-tapped timing: PASS - correctly applied at ETB via on_enter_battlefield
- Mana ability availability when tapped: PASS - correctly requires untapped state
- Two separate mana choices (G or U): PASS - provides two distinct ManaAbilityDef options

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Subtype checking for regular vs token lands: NOT TESTED
- Self-exclusion behavior: NOT TESTED  
- Controller ownership check: NOT TESTED
- Zone filtering to Battlefield: NOT TESTED
- Enters-tapped timing: NOT TESTED
- Enters untapped with Forest present: NOT TESTED
- Enters untapped with Island present: NOT TESTED
- Mana ability functionality: NOT TESTED
- Card data correctness: `innistrad_simple_cards.rs:91` (basic card type check only)