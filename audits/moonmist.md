## Audit — 2026-04-01

**Scryfall Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Scryfall type line**: Instant
**Status**: ISSUE

- Name: Correct ("Moonmist")
- Cost: {1}{G} - Correct
- Type: Instant - Correct
- Oracle text matches.

Issues:
1. **Combat damage prevention not implemented**: The card's second effect ("Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves") is explicitly noted as not implemented in the code comments. This is a significant portion of the card's functionality. The code only transforms Human DFCs.
2. **Transform scope**: Oracle says "Transform all Humans" -- the implementation only transforms Humans that have a back face (DFCs). This is a reasonable simplification since non-DFC Humans can't transform, but the Oracle text technically means all Humans (and in Innistrad, Human Werewolves are the relevant ones). This is acceptable.

- Tests: innistrad_simple_cards.rs has `moonmist_card_data` test (card data only, no functional test for transform or damage prevention).

## Audit — 2026-04-01

**Scryfall Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Scryfall type line**: Instant
**Status**: ISSUE

1. Combat damage prevention for non-Werewolf/Wolf creatures is not implemented. The code acknowledges this in a comment (line 80) but it is a significant missing feature. (File: /home/user/mtg-imsaho/mtg-engine/src/cards/moonmist.rs, line 80)
2. Per Scryfall ruling, Moonmist transforms ALL double-faced Humans, not just Werewolves. The implementation already handles this correctly by checking for Human subtype + back face.
