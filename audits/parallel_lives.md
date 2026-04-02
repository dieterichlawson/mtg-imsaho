# Audit: Parallel Lives

## Reference (Scryfall/API)
- **Name:** Parallel Lives
- **Mana Cost:** {3}{G}
- **Type:** Enchantment
- **Oracle:** If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.

## Implementation: `parallel_lives.rs`
- **Name:** Parallel Lives -- CORRECT
- **Mana Cost:** {3}{G} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None -- CORRECT
- **oracle_text field:** Matches oracle -- CORRECT
- **Behavior:** Token doubling is handled externally in GameState::create_token_with_subtypes (per code comment) -- CORRECT approach for a replacement effect

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
**Type line**: Enchantment
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {3}{G}, type Enchantment, token-doubling replacement effect. Behavior is correctly delegated to the engine's token creation logic.
