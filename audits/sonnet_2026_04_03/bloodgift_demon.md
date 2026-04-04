## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Type line**: Creature — Demon
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "target player" includes all players (including self): PASS - code correctly iterates through all players who haven't lost
- Draw and life loss happen to same player: PASS - DrawAndLoseLife effect uses same player ID for both operations
- Draw happens before life loss: PASS - engine executes draw_cards() before life modification
- Trigger only fires on controller's upkeep: PASS - code checks `state.active_player != controller`
- Trigger resolves even if demon leaves battlefield: PASS - standard triggered ability behavior, no source dependency in effect
- Targeting choice is presented to player: PASS - creates AwaitingAction::ResolutionChoice with player selection

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic upkeep trigger functionality: `mtg-engine/tests/tier7_cards.rs:70` - bloodgift_demon_draws_and_loses_life()
- Player can target self: `mtg-engine/tests/tier7_cards.rs:70` - test chooses P0 to target P0
- Card draw verification: `mtg-engine/tests/tier7_cards.rs:96` - asserts hand count increases by 1
- Life loss verification: `mtg-engine/tests/tier7_cards.rs:97` - asserts life decreases from 20 to 19
- Multiple player targeting options: NOT TESTED
- Demon leaving battlefield after trigger: NOT TESTED
- Empty game state (no valid targets): NOT TESTED