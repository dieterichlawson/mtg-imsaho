## Audit — 2026-04-01

**Scryfall Oracle text**: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
**Scryfall type line**: Enchantment
**Status**: PASS

- Name: Correct ("Parallel Lives")
- Cost: {3}{G} - Correct
- Type: Enchantment - Correct
- Oracle text matches.
- Implementation: The card data itself is minimal (just an enchantment on the battlefield). The actual doubling logic is handled in `GameState::create_token_with_subtypes` as noted in the comments.
- Tests: tier14_cards.rs has `parallel_lives_doubles_tokens`, `no_parallel_lives_single_token`, and `parallel_lives_only_doubles_for_controller`. Good coverage including the "your control" restriction.

No issues found.
