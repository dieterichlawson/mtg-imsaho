## Audit — 2026-04-01

**Scryfall Oracle text**: Exile target card from a graveyard.\nFlashback {W}
**Scryfall type line**: Instant
**Status**: PASS

- Name: Correct ("Purify the Grave")
- Cost: {W} - Correct
- Type: Instant - Correct
- Flashback: {W} - Correct
- Oracle text matches.
- Target: GraveyardCard - Correct
- on_resolve: Exiles target card from graveyard. Correct.
- Tests: tier11_cards.rs has `purify_the_grave_exiles_card_from_graveyard` and `purify_the_grave_has_flashback`.

No issues found.
