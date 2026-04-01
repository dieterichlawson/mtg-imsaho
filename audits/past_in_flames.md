## Audit — 2026-04-01

**Scryfall Oracle text**: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Correct ("Past in Flames")
- Cost: {3}{R} - Correct
- Type: Sorcery - Correct
- Flashback: {4}{R} - Correct
- Oracle text matches.
- Implementation: Moves itself to graveyard first (via move_spell_after_resolve), then iterates over all instant/sorcery cards in the controller's graveyard and grants flashback with cost equal to their mana cost via `until_end_of_turn_flashback`. Correct.
- Skips cards that already have flashback granted. Correct.
- Tests: tier14_cards.rs has `past_in_flames_grants_flashback_to_all`.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. Flashback {4}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Implementation correctly grants flashback to graveyard instants/sorceries and has its own flashback cost.
