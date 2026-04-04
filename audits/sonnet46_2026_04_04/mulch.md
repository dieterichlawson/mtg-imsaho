## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Fewer than 4 cards in library**: Code uses `std::cmp::min(4, player.library_order.len())` so it correctly reveals as many as available. PASS.
- **Empty library**: `count = 0`, `revealed` is empty, both loops do nothing, `move_spell_after_resolve` is still called. PASS.
- **All revealed cards are lands**: Non-lands loop is empty, all cards go to hand. PASS.
- **All revealed cards are non-lands**: Lands loop is empty, all cards go to graveyard. PASS.
- **Reveal is public information**: Card names are logged before zone changes via `state.log(LogLevel::Event, format!("Mulch revealed: {}", ...))`. PASS.
- **Library card zone tracking consistency**: `player.library_order.drain(..count)` removes cards from the ordered library tracking, then `move_object` updates each card's `zone` field. No double-removal or orphaned references. PASS.
- **Controller vs. owner**: Code correctly uses `controller` (from the Mulch stack object) for "your library" — correct per oracle text. PASS.
- **Land detection for real cards**: Uses `registry.card_data(o.card_id).map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)))`. Library cards are always real cards (not tokens), so registry lookup is sufficient. PASS.
- **Token land concern**: Tokens cannot exist in a library, so checking only registry card data is correct here. PASS.
- **move_spell_after_resolve**: Called at end of `on_resolve`; correctly sends Mulch to exile if cast with flashback, graveyard otherwise. PASS.
- **Mana cost**: `{1}{G}` encoded as `Generic(1)` + `Colored(Color::Green)`. PASS.
- **Card type**: `vec![CardType::Sorcery]`. PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic case (2 lands + 2 non-lands in 4-card library, lands go to hand, non-lands to graveyard): `mtg-engine/tests/tier11_cards.rs:219` — TESTED
- Library with fewer than 4 cards: NOT TESTED
- Empty library: NOT TESTED
- All 4 revealed cards are lands: NOT TESTED
- All 4 revealed cards are non-lands: NOT TESTED
- Mulch cast with flashback goes to exile: NOT TESTED
