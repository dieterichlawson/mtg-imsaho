# Audit: Laboratory Maniac

## Oracle (Official)
- **Name:** Laboratory Maniac
- **Cost:** {2}{U}
- **Type:** Creature — Human Wizard
- **Oracle:** If you would draw a card while your library has no cards in it, you win the game instead.
- **P/T:** 2/2
- **Source:** Scryfall API (cached 2026-04-01)

## Key Rulings
1. This is a **replacement effect** (static ability), NOT a triggered ability. It replaces the draw event itself.
2. Laboratory Maniac must be on the battlefield at the exact moment the draw would occur.
3. Milling is not drawing -- Lab Maniac does not replace milling from an empty library.
4. The win happens mid-resolution of whatever caused the draw. The game ends immediately.
5. If you cannot win (e.g. Angel's Grace), the draw is still replaced and you do not lose from the empty library draw.

## Implementation Review

### Card Data (`mtg-engine/src/cards/isd/laboratory_maniac.rs`)
- Name: `"Laboratory Maniac"` -- CORRECT
- Cost: `Generic(2), Colored(Blue)` = {2}{U} -- CORRECT
- Types: `Creature` -- CORRECT
- Subtypes: `["Human", "Wizard"]` -- CORRECT
- P/T: `2/2` -- CORRECT
- Oracle text string: matches verbatim -- CORRECT
- `triggered_abilities: vec![]` -- CORRECT (this is NOT a triggered ability)
- No replacement effect registered on the card struct itself -- acceptable given the engine-level implementation

### Replacement Effect Logic (`mtg-engine/src/engine.rs`, lines 2327-2350)
The replacement effect is implemented directly inside `draw_cards()`:

```rust
None => {
    let has_lab_maniac = state.objects.values().any(|o| {
        o.zone == Zone::Battlefield
            && o.controller == player
            && o.name == "Laboratory Maniac"
    });
    if has_lab_maniac {
        state.get_player_mut(player).has_drawn_from_empty = false;
        let opponent = state.opponent(player);
        state.players[opponent.0 as usize].lost = true;
        state.events.push(GameEvent::PlayerLost {
            player: opponent,
            reason: crate::events::LossReason::LifeReachedZero, // closest reason
        });
        state.result = Some(crate::state::GameResult::Winner(player));
    }
    break;
}
```

**Architecture:** The effect is hardcoded in the engine's `draw_cards` function rather than being a modular replacement effect on the card. This is a pragmatic approach that correctly implements the behavior as a replacement (the draw is replaced before it happens), though it is not generalizable to other replacement effects.

### Tests (`mtg-engine/tests/tier14_cards.rs`)
Three tests exist:
1. `laboratory_maniac_wins_on_empty_library_draw` -- verifies win on empty draw -- PASS
2. `no_lab_maniac_loses_on_empty_draw` -- verifies normal loss without Lab Maniac -- PASS
3. `laboratory_maniac_only_helps_controller` -- verifies it does not help opponents -- PASS

### LLM Knowledge (`mtg-player/src/llm.rs`)
No mention of Laboratory Maniac found. Not a blocking issue since the LLM does not need special knowledge for this card.

## Issues

1. **ISSUE (minor): Wrong `LossReason` for opponent loss.**
   The code uses `LossReason::LifeReachedZero` with the comment `// closest reason` when the opponent loses because the Lab Maniac player won. This is semantically incorrect. The opponent did not lose because their life reached zero; they lost because the game was won by an opponent. A `LossReason::OpponentWon` or similar variant would be more accurate for logging/debugging purposes.

   Code (`engine.rs` line 2343):
   ```rust
   reason: crate::events::LossReason::LifeReachedZero, // closest reason
   ```

2. **ISSUE (minor): Hardcoded card name in engine.**
   The replacement effect is hardcoded by name (`o.name == "Laboratory Maniac"`) in the core `draw_cards` function rather than being driven by card data (e.g., a replacement effect registry). This means adding similar cards (e.g., Jace, Wielder of Mysteries; Thassa's Oracle) would require additional hardcoding in the engine. Not a correctness bug, but a maintainability concern.

3. **ISSUE (minor): No test for multiple-draw scenario.**
   If a spell says "draw 3 cards" and the library has 1 card left, Lab Maniac should let you draw the first card normally, then win the game on the second draw attempt. The current `break` on line 2350 correctly stops drawing after the win, but there is no test verifying this specific scenario.

4. **ISSUE (minor): Multiplayer assumption.**
   The code calls `state.opponent(player)` and marks a single opponent as lost. In a multiplayer game, winning means all opponents lose, not just one. This is acceptable if the engine only supports 2-player games.

## Verdict: PASS (with minor issues)

The replacement effect is correctly implemented as a replacement (not a triggered ability). It fires at the right time (when a draw from an empty library would occur), replaces the draw (the player does not lose), and immediately wins the game. Card data is accurate. The four minor issues noted above do not affect correctness for 2-player games.
