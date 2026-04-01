## Audit — 2026-04-01

**Scryfall Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.\nFlashback {6}{B}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {2}{U}: correct.
- Type Instant: correct.
- Flashback {6}{B}: correct.
- Reveals top 4 cards, player picks 1 for hand, rest to graveyard: correct.
- When only 1 card, auto-puts in hand: correct.
- When 2+ cards, presents ChooseFromRevealed choice: correct.
- Uses `move_spell_after_resolve` (deferred when choice is pending): correct.
- Tests exist in `flashback.rs` (`forbidden_alchemy_draws_and_mills`) and `card_mechanics.rs` (`forbidden_alchemy_choice_from_top_4`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard. Flashback {6}{B}
**Scryfall type line**: Instant
**Status**: ISSUE

1. **LLM card knowledge inaccurate**: The LLM entry says "Draw 1 card, mill 3" but the Oracle text is "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard." This is NOT draw (doesn't trigger "whenever you draw" effects) and the player CHOOSES which card to keep. File: `mtg-player/src/llm.rs`, line 113.
