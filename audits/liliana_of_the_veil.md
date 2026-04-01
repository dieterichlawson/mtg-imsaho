## Audit — 2026-04-01

**Scryfall Oracle text**: +1: Each player discards a card.\n-2: Target player sacrifices a creature.\n-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of your choice.
**Scryfall type line**: Legendary Planeswalker — Liliana
**Status**: ISSUE

**Findings**:

1. Name: Liliana of the Veil -- correct
2. Cost: {1}{B}{B} -- correct
3. Type: Planeswalker with Legendary supertype -- correct
4. Subtypes: Liliana -- correct
5. Starting loyalty: 3 -- correct
6. **+1 ability**: Each player discards a card -- implemented but simplified (auto-picks first card in hand rather than letting each player choose). Acceptable simplification but noted.
7. **-2 ability**: "Target player sacrifices a creature" -- implementation hardcodes opponent as the target rather than allowing targeting any player. The Oracle text says "target player" which means it could target yourself. Minor issue.
8. **-6 ability**: Pile separation is simplified to "sacrifice half their permanents" targeting opponent only. This is a significant simplification but noted as acceptable given engine constraints. The Oracle text allows targeting any player and requires actual pile division.
9. Tests: no dedicated test file found for Liliana.

**Summary**: The -2 ability should allow targeting any player (not just opponent). The +1 discard should ideally let each player choose which card to discard. These are simplifications rather than bugs.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: +1: Each player discards a card. -2: Target player sacrifices a creature. -6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of your choice.
**Scryfall type line**: Legendary Planeswalker -- Liliana
**Status**: ISSUE

- Confirmed issues from previous audit. Additionally:
- Tests DO exist in tier15_cards.rs (liliana_enters_with_loyalty, liliana_plus_one_each_player_discards, liliana_minus_two_opponent_sacrifices_creature, and several planeswalker SBA tests).
- The on_resolve correctly moves to battlefield and sets starting loyalty via add_counters.
- +1 Scryfall ruling: "first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at the same time." Current implementation discards sequentially (auto-picks first card for each player). This is a simplification.
- -2 correctly uses crate::destruction::sacrifice (not try_destroy) -- this is correct per Oracle "sacrifices."
