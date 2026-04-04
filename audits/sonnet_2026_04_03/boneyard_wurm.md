## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Type line**: Creature — Wurm
**Status**: ISSUE

### Code issues
- Incomplete creature card identification in dynamic_pt method (`mtg-engine/src/cards/isd/boneyard_wurm.rs:36`)
  - Oracle text says: `creature cards in your graveyard`
  - Code does: `filter(|o| o.power.is_some())` - only checks for objects with power, missing creature cards that don't have power set on the object but are creature cards according to their registry card type. Engine uses comprehensive check elsewhere: `o.power.is_some() || registry.card_data(o.card_id).map(|d| d.card_types.contains(&CardType::Creature)).unwrap_or(false)`

### Tricky interactions checked
- Creature card identification: FAIL - only checks power.is_some(), should use comprehensive creature card check like engine.rs does
- Power equals toughness: PASS - returns (count, count) correctly
- Works in all zones: PASS - dynamic_pt doesn't restrict by zone, matching the ruling that ability works in all zones
- Counts itself in graveyard: PASS - no self-exclusion in filter, correctly includes itself per ruling
- Controller-based graveyard: PASS - uses object.controller to determine "your graveyard"
- Dynamic updates: PASS - dynamic_pt called when P/T needed, continuously evaluates

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic creature counting: `mtg-engine/tests/tier7_cards.rs:19-37` / TESTED
- Counts itself when in graveyard: NOT TESTED
- Works in zones other than battlefield: NOT TESTED
- Creature cards without power.is_some() but with CardType::Creature: NOT TESTED
- Controller change affecting graveyard reference: NOT TESTED
- Mixed creature cards (some with power, some registry-only): NOT TESTED