## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Each creature you control"** scope: PASS - Code correctly identifies controller of Gavony Township, finds all creatures controlled by that player on the battlefield using `objects_in_zone(Zone::Battlefield, controller)`, and filters for creatures using `o.power.is_some()` which is the consistent pattern throughout the engine
- **Counter type correctness**: PASS - Uses `CounterType::PlusOnePlusOne` which correctly represents +1/+1 counters
- **Activated ability timing**: PASS - Ability can be activated at instant speed (`sorcery_speed_only: false`) and requires tapping the land (`requires_tap: true`)
- **Mana ability functionality**: PASS - Mana ability adds {C} and requires tap, only available when untapped on battlefield
- **No targeting required**: PASS - Ability correctly uses `target_requirement: None` since it affects all creatures you control without targeting

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic card data (land type, no mana cost): `mtg-engine/tests/tier10_cards.rs:223-229` (gavony_township_card_data)
- Activated ability puts +1/+1 counters on controller's creatures: `mtg-engine/tests/tier10_cards.rs:231-263` (gavony_township_counters_all_creatures)
- Ability doesn't affect opponent's creatures: `mtg-engine/tests/tier10_cards.rs:231-263` (gavony_township_counters_all_creatures)
- Mana ability: NOT TESTED

Sources:
- [Gavony Township rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Gavony-Township/rulings/)
- [Phasing - MTG Wiki - Fandom](https://mtg.fandom.com/wiki/Phasing)