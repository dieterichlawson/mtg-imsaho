## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If you would draw a card while your library has no cards in it, you win the game instead.
**Type line**: Creature — Human Wizard
**Status**: PASS

### Code issues

No issues found.

The replacement effect is not implemented in the card's `CardBehavior` trait (the card file has only `card_data()`), but is instead hardcoded into the engine's `draw_cards()` function in `mtg-engine/src/engine.rs` (lines 2718–2741). The effect is functionally correct per oracle text: when `draw_top_card()` returns `None` (library is empty) for a player who controls an object on the battlefield named `"Laboratory Maniac"`, the engine clears the `has_drawn_from_empty` flag (preventing SBA loss) and sets `state.result = Some(GameResult::Winner(player))`. The player wins the game as the oracle requires.

**Noted but not flagged — cosmetic bug**: The `PlayerLost` event emitted for the opponent uses `reason: crate::events::LossReason::LifeReachedZero` (with comment `// closest reason`). The opponent's life did not reach zero; the correct semantic is that the drawing player won the game. There is a `LossReason::DrewFromEmptyLibrary` variant that is marginally more accurate, but neither that nor any existing variant perfectly describes "opponent wins via Laboratory Maniac." The `LossReason` field is never read by game-control logic — it appears only in the `GameEvent::PlayerLost` variant, which is used for logging only. Game outcome is determined by `state.result` and `is_game_over()`, both of which are set correctly. This is a cosmetic inaccuracy in event metadata.

**Noted but not flagged — missing `GameEnded` event**: When Lab Maniac wins, `draw_cards()` sets `state.result` directly. SBA, which runs afterward, skips the `GameEnded` emission because it guards with `state.result.is_none()` (`sba.rs` line 319). The `GameEnded` event is therefore never pushed for Lab Maniac wins. The engine loop exits via `is_game_over()` checking `state.result.is_some()`, so the game does end correctly. No consumer uses the `GameEnded` event for control flow — the runner checks `state.result` directly. This is a structural gap in the event log, not a behavioral issue.

### Tricky interactions checked

- **Library-empty detection**: `draw_top_card()` returns `None` when `library_order.is_empty()`, setting `has_drawn_from_empty = true`. The `None` arm in `draw_cards()` correctly triggers the Lab Maniac check. pass
- **Controlling player check**: The check `o.controller == player` correctly ensures only the player controlling Lab Maniac gets the replacement, not an opponent who might control one. pass (verified by `laboratory_maniac_only_helps_controller` test)
- **Battlefield zone check**: `o.zone == Zone::Battlefield` is present; a Lab Maniac in hand, graveyard, or library does not trigger the win. pass
- **`has_drawn_from_empty` cleared before game ends**: The flag is set to `false` before `state.result` is set, so SBA can never see both `has_drawn_from_empty = true` and then fire a simultaneous library-draw loss for the winning player. pass
- **All draw paths go through `draw_cards()`**: Searched all callers of `draw_top_card()` across the codebase. The only non-engine caller is `mirror_mad_phantasm.rs`, which uses it for the "reveal until you find" mechanic — not an actual card draw. All actual draw effects (`divination.rs`, `curiosity.rs`, `think_twice.rs`, `murder_of_crows.rs`, `desperate_ravings.rs`, `mentor_of_the_meek.rs`, etc.) call `crate::engine::draw_cards()`, so Lab Maniac fires for all of them. pass
- **Draw step**: The turn-draw at `engine.rs:2959` calls `draw_cards(state, active, 1)`, so the Lab Maniac check is active for the mandatory draw step. pass
- **Multi-card draw stops at first empty draw**: When drawing N cards (e.g., `draw_cards(state, p, 3)`) and the library empties mid-draw, the `break` statement exits the loop after the replacement fires. Remaining draws from the count do not re-trigger the check. This is correct — the replacement applies once (the game ends). pass
- **Multiple Lab Maniacs**: The check `state.objects.values().any(|o| ...)` fires if at least one Lab Maniac is on the battlefield. Per oracle text only one is needed. pass
- **No summoning-sickness requirement**: Lab Maniac's ability is a static replacement effect on the card, not an activated or triggered ability. The code correctly checks existence on the battlefield with no tap/sick restriction. pass

### Test coverage

- **Basic Lab Maniac win on empty library draw**: `mtg-engine/tests/tier14_cards.rs` — `laboratory_maniac_wins_on_empty_library_draw` (line 20). TESTED
- **No Lab Maniac → player loses from empty draw**: `mtg-engine/tests/tier14_cards.rs` — `no_lab_maniac_loses_on_empty_draw` (line 41). TESTED
- **Lab Maniac only helps its controller, not opponent**: `mtg-engine/tests/tier14_cards.rs` — `laboratory_maniac_only_helps_controller` (line 61). TESTED
- **Lab Maniac on battlefield (not in hand/graveyard)**: NOT TESTED — no test verifies that a Lab Maniac in graveyard or hand does not trigger the win condition.
- **Lab Maniac with multi-card draw (N > 1, library has some cards)**: NOT TESTED — no test for drawing 3 with 2 cards in library to verify Lab Maniac fires on the third draw.
- **`LossReason::LifeReachedZero` emitted for Lab Maniac win**: NOT TESTED — the incorrect loss reason is not covered by any assertion.
- **`GameEnded` event absent for Lab Maniac win**: NOT TESTED — no test inspects the events list for presence/absence of `GameEnded`.
