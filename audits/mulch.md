## Audit — 2026-04-01

**Scryfall Oracle text**: Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Correct ("Mulch")
- Cost: {1}{G} - Correct
- Type: Sorcery - Correct
- Oracle text matches.
- Implementation: Reveals top 4 cards (handles case where library has fewer), sorts into lands (to hand) and non-lands (to graveyard). Correct.
- Uses registry card_data to check CardType::Land for land detection. Correct.
- Logs reveal, logs lands put into hand. Correct.
- Tests: tier11_cards.rs has `mulch_puts_lands_in_hand_and_rest_in_graveyard`.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Reveals top 4, separates lands from non-lands, lands to hand, rest to graveyard. Uses move_spell_after_resolve.
