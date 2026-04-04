## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Creature card identification in graveyard: PASS - uses `o.power.is_some()` which is the engine's standard approach for identifying creature cards, confirmed by usage in Wreath of Geists, Splinterfright, and test setup
- Self-exclusion when cast from graveyard: PASS - code includes `o.id != object_id` to exclude the resolving spell from the creature count
- Life gain calculation and events: PASS - correctly calculates `creature_count * 2`, updates player life, and emits LifeChanged event
- Flashback cost and timing: PASS - card_data declares correct flashback_cost {2}{G}, engine handles instant-speed casting from graveyard
- Flashback exile after resolution: PASS - uses `move_spell_after_resolve()` which checks `cast_with_flashback` flag and moves to exile vs graveyard appropriately
- Zero creatures edge case: PASS - code checks `if life_gain > 0` before modifying life or emitting events
- Graveyard ownership verification: PASS - filters by `o.owner == controller` to check controller's graveyard only

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic life gain functionality (3 creatures = 6 life): `mtg-engine/tests/flashback.rs:323-344`
- Flashback spell resolution and exile: `mtg-engine/tests/flashback.rs:84-106` (general flashback mechanics)
- Flashback cost verification: `mtg-engine/tests/flashback.rs:22-40` (general flashback mechanics)
- Flashback timing restrictions: `mtg-engine/tests/flashback.rs:42-61` (general flashback mechanics)
- Flashback exile after being countered: `mtg-engine/tests/flashback.rs:127-162` (general flashback mechanics)
- Zero creatures in graveyard: NOT TESTED
- Self-exclusion when cast from graveyard: NOT TESTED
- Graveyard vs exile distinction for flashback: NOT TESTED (for this specific card)

Sources:
- [Gnaw to the Bone MTG - Innistrad #183 (English) | Magic: The Gathering](https://gatherer.wizards.com/ISD/en-us/183/gnaw-to-the-bone)
- [Gnaw to the Bone rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Gnaw-to-the-Bone/rulings/)
- [Gnaw to the Bone · Innistrad (ISD) #183](https://scryfall.com/card/isd/183/gnaw-to-the-bone)