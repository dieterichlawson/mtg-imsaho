## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Multi-type cards (e.g., artifact creatures): pass (uses .contains() so artifact creatures match both "Artifact" and "Creature" choices)
- Flashback exile behavior: pass (move_spell_after_resolve correctly exiles when cast_with_flashback is true)
- Player choice requirement: pass (presents choice of all 5 permanent types via ChooseCardType resolution mechanism)
- "All cards" vs targeting: pass (returns ALL matching cards without targeting)
- Graveyard scope: pass (filters by controller and Zone::Graveyard)
- Keywords field vs flashback_cost field: pass (Flashback is not defined in Keyword enum, functionality correctly implemented via flashback_cost field)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Choose permanent type and return matching cards: `tier15_cards.rs:467` (creeping_renaissance_returns_creatures_from_graveyard)
- Type selectivity (only chosen type returned): `tier15_cards.rs:506` (creeping_renaissance_only_returns_chosen_type)
- Flashback exile behavior: `tier15_cards.rs:553` (creeping_renaissance_flashback_exiles)
- Multi-type cards handling: NOT TESTED (but implementation is correct with .contains())
- All 5 permanent type options available: NOT TESTED (but implementation lists all 5 types correctly)