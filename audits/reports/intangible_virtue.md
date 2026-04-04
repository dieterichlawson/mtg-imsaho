# Audit: Intangible Virtue

## Oracle (Official)
- **Name:** Intangible Virtue
- **Cost:** {1}{W}
- **Type:** Enchantment
- **Oracle:** Creature tokens you control get +1/+1 and have vigilance.
- **P/T:** N/A

## Implementation
- Name: "Intangible Virtue" -- CORRECT
- Cost: {1}{W} -- CORRECT
- Type: Enchantment -- CORRECT
- Oracle text matches -- CORRECT
- Continuous effects:
  - ModifyPT { power: 1, toughness: 1, scope: Global(YourTokens) } -- CORRECT
  - GrantKeyword { keyword: Vigilance, scope: Global(YourTokens) } -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Intangible Virtue
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Enchantment
- **Cost:** {1}{W}
- **Oracle:** Creature tokens you control get +1/+1 and have vigilance.

### Card Data
- **Name:** Intangible Virtue -- PASS
- **Cost:** {1}{W} -- PASS
- **Types:** Enchantment -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Exact match. -- PASS

### Behavior Audit
- **Continuous effect +1/+1:** ModifyPT with power:1 toughness:1, scope YourTokens. -- PASS
- **Continuous effect vigilance:** GrantKeyword Vigilance, scope YourTokens. -- PASS
- **Scope:** Both effects correctly scoped to YourTokens (creature tokens you control). -- PASS

### Result: PASS

## Audit — 2026-04-03 07:04
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/19/intangible-virtue)
**Oracle text**: Creature tokens you control get +1/+1 and have vigilance.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
1. **LLM card knowledge is wrong** (`mtg-player/src/llm.rs` line 105): The AI player description reads `"Intangible Virtue ({1}{W} enchantment): Your creatures get +1/+1."` which is incorrect in two ways:
   - Says "Your creatures" instead of "Creature tokens you control" -- omits the critical tokens-only restriction.
   - Omits "and have vigilance" entirely.
   This causes the AI player to misunderstand the card, potentially leading to incorrect play decisions (e.g., casting it to buff non-token creatures, not factoring in vigilance for attack/block decisions).

### Tricky interactions checked (min 3)
1. **Token vs non-token filtering**: `CreatureFilter::YourTokens` in `state.rs:653` correctly checks both `creature.controller == source_controller && creature.is_token`. Tests confirm non-tokens are not buffed.
2. **Opponent's tokens not buffed**: The `YourTokens` filter requires `creature.controller == source_controller`, so an opponent's tokens are correctly excluded from the buff. No dedicated test exists for this, but the filter logic is sound.
3. **Multiple Intangible Virtues stacking**: The engine iterates all battlefield sources in `continuous_pt_mods` (state.rs:726-757), so two copies would each independently apply +1/+1 for a total of +2/+2. This is correct MTG behavior and follows from the general continuous effect architecture.
4. **Removal of enchantment stops buff**: Since the engine re-computes continuous effects each time via `effective_power`/`effective_toughness`/`has_keyword`, removing Intangible Virtue from the battlefield immediately stops the buff. Correct behavior.

### Test coverage
- `intangible_virtue_token_only` (card_mechanics.rs:336): Tests token gets +1/+1 and vigilance, non-token does not. PASS.
- `intangible_virtue_buffs_creatures` (tier3_cards.rs:488): Tests token gets +1/+1 and vigilance, non-token is unaffected. PASS.
- Both tests pass. Coverage is adequate for the core behavior but does not test opponent-token exclusion.
