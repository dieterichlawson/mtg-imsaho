## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
**Type line**: Land
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 21:03
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/239/gavony-township)
**Oracle text**: {T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
**Type line**: Land
**Status**: PASS
### Code issues
No issues found. Implementation matches oracle text exactly.
### Tricky interactions checked (min 3)
1. **Only controller's creatures get counters**: `objects_in_zone(Zone::Battlefield, controller)` correctly scopes to the activating player's creatures. Test verifies opponent's creature does NOT receive a counter.
2. **Gavony Township itself does not receive a counter**: The land has `power: None`, so `power.is_some()` excludes it from the counter loop. Correct behavior.
3. **Mana ability vs activated ability separation**: Tap-for-colorless is a `ManaAbilityDef` (doesn't use the stack), while the {2}{G}{W},{T} counter ability is an `ActivatedAbilityDef` (uses the stack). Correct per MTG rules.
4. **Instant-speed activation**: `sorcery_speed_only: false` is correct; the card does not restrict activation timing.
### Test coverage
- `gavony_township_card_data`: Validates card type is Land and cost is None.
- `gavony_township_counters_all_creatures`: Verifies +1/+1 counters placed on controller's two creatures, and NOT on opponent's creature. Both tests pass.
