## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues
- Missing "may" choice implementation in `mtg-engine/src/cards/isd/ghost_quarter.rs:81`
  - Oracle text says: `Its controller may search their library for a basic land card`
  - Code does: Automatic search without presenting the choice to the player (comment: "auto-search")
- Missing shuffle implementation in `mtg-engine/src/cards/isd/ghost_quarter.rs:69-103`
  - Oracle text says: `put it onto the battlefield, then shuffle`
  - Code does: No shuffling after putting the land onto the battlefield

### Tricky interactions checked
- Indestructible land targeted: Pass (controller still gets to search per ruling, code correctly doesn't check destroy result)
- Regenerated land targeted: Pass (controller still gets to search per ruling, code correctly doesn't check destroy result)  
- Illegal target by resolution time: Pass (code returns early if target not on battlefield, matches ruling)
- Self-targeting: Pass (would return early due to sacrifice happening before resolution)
- No basic land in library: Pass (code handles empty search gracefully)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic card data and mana ability: `mtg-engine/tests/innistrad_simple_cards.rs:152-172`
- Land destruction ability: NOT TESTED
- "May search" choice: NOT TESTED
- Search when target indestructible/regenerated: NOT TESTED
- Illegal target handling: NOT TESTED
- Library shuffling: NOT TESTED