## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Each player mills a card.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Each player" affects all players (not targeting): pass - code correctly iterates over `state.players.iter().map(|p| p.id).collect()` at lines 51-52
- "mills a card" uses correct mill count of 1: pass - code calls `mill_cards(state, pid, 1)` at line 53  
- {T} tap cost correctly implemented: pass - `requires_tap: true` and checks `!obj.tapped` condition at line 34
- Activated ability only available when untapped and on battlefield: pass - condition `obj.zone == Zone::Battlefield && !obj.tapped` at line 34
- Mill function correctly moves cards from library to graveyard: pass - `mill_cards` function in engine.rs moves cards with `state.move_object(card_id, Zone::Graveyard)` at line 2765

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (mana cost, type): `innistrad_simple_cards.rs:337` (ghoulcallers_bell_card_data)
- Mills both players when activated: `innistrad_simple_cards.rs:346` (ghoulcallers_bell_mills_both_players)
- Basic mill functionality: NOT TESTED (assumes engine mill_cards function works)
- Empty library handling during mill: NOT TESTED
- Tap cost enforcement: NOT TESTED (assumes engine handles tap costs)