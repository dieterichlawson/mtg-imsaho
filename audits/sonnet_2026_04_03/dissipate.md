## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Implementation doesn't check if counter attempt succeeds before exiling (`mtg-engine/src/cards/isd/dissipate.rs:50-51`)
  - Oracle text says: `If that spell is countered this way, exile it instead of putting it into its owner's graveyard.`
  - Code does: Unconditionally removes spell from stack and moves to exile without checking if counter succeeded
- Log message is inaccurate (`mtg-engine/src/cards/isd/dissipate.rs:52`)
  - Oracle text says: Counter attempt may fail, so exile is conditional
  - Code does: Always logs "was countered and exiled" even if spell can't be countered

### Tricky interactions checked
- "If that spell is countered this way": FAIL - code doesn't check if counter succeeded, always exiles
- "Counter target spell": PASS - correctly targets spells on stack only
- "exile it instead of putting it into its owner's graveyard": FAIL - should only happen if counter succeeds
- Stack manipulation vs proper countering: FAIL - directly manipulates stack instead of using counter logic
- Uncounterable spells interaction: FAIL - would incorrectly exile uncounterable spells per ruling

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic counter and exile behavior: `mtg-engine/tests/tier2_spells.rs:63` 
- "If that spell is countered this way" conditional: NOT TESTED
- Uncounterable spells don't get exiled: NOT TESTED  
- "The card does not go to the graveyard before being exiled" ruling: NOT TESTED
- Proper failure when spell can't be countered: NOT TESTED

**Sources:**
- [Dissipate MTG rulings and mechanics discussions](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/306128-differencing-between-exile-target-spell-and)
- [Dissipate card database entries](https://scryfall.com/card/isd/53/dissipate)