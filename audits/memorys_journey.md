## Audit — 2026-04-01

**Scryfall Oracle text**: Target player shuffles up to three target cards from their graveyard into their library.\nFlashback {G}
**Scryfall type line**: Instant
**Status**: ISSUE

**Findings**:

1. Name: Memory's Journey -- correct
2. Cost: {1}{U} -- correct
3. Type: Instant -- correct
4. Flashback: {G} -- correct
5. **ISSUE — Targeting semantics simplified**: The Oracle text has two targets: "target player" and "up to three target cards from their graveyard." The cards must come from the targeted player's graveyard specifically. The implementation uses `UpToTargets(3, GraveyardCard)` which targets up to 3 graveyard cards from any player's graveyard without linking them to a specific targeted player. This could allow targeting cards from different players' graveyards simultaneously, which is incorrect per Oracle text.
6. The implementation shuffles ALL players' libraries rather than just the targeted player's library.
7. Tests exist in tier11_cards.rs (basic functionality and flashback).

**Summary**: The targeting model is simplified -- it should target a specific player and then only allow targeting cards in that player's graveyard. Only that player's library should be shuffled.
