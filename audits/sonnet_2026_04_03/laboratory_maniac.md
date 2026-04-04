## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If you would draw a card while your library has no cards in it, you win the game instead.
**Type line**: Creature — Human Wizard
**Status**: PASS

### Code issues

No issues found.

### Tricky interactions checked

- **"would draw a card"**: Replacement effect triggers correctly when `draw_top_card()` returns `None` due to empty library - PASS
- **"while your library has no cards in it"**: Condition properly checked by `library_order.is_empty()` in `draw_top_card()` method - PASS 
- **"you win the game instead"**: Correctly sets `state.result = Some(GameResult::Winner(player))` and marks opponent as lost - PASS
- **Controller requirement**: Implementation correctly checks `o.controller == player` so only the Lab Maniac's controller benefits - PASS
- **Battlefield requirement**: Implementation correctly checks `o.zone == Zone::Battlefield` - PASS
- **Multiple Lab Maniacs**: Uses `.any()` so works with any number of Lab Maniacs controlled by player - PASS
- **Replacement effect timing**: Fires at correct moment when draw would occur from empty library, before SBA check - PASS
- **Flag management**: Correctly clears `has_drawn_from_empty` flag so SBA doesn't cause loss after win - PASS

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- **Basic win condition (empty library draw with Lab Maniac)**: `mtg-engine/tests/tier14_cards.rs:20` (`laboratory_maniac_wins_on_empty_library_draw`)
- **Normal loss without Lab Maniac**: `mtg-engine/tests/tier14_cards.rs:41` (`no_lab_maniac_loses_on_empty_draw`)
- **Controller-only benefit**: `mtg-engine/tests/tier14_cards.rs:61` (`laboratory_maniac_only_helps_controller`)
- **Replacement effect (not triggered ability)**: NOT TESTED (but implementation correctly uses replacement effect in draw_cards, not triggered_abilities)
- **Must control at draw time**: NOT TESTED 
- **Multiple simultaneous draws**: NOT TESTED
- **Interaction with other replacement effects**: NOT TESTED
