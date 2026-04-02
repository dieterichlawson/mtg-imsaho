# Audit: Night Terrors

## Reference (Scryfall/API)
- **Name:** Night Terrors
- **Mana Cost:** {2}{B}
- **Type:** Sorcery
- **Oracle:** Target player reveals their hand. You choose a nonland card from it. Exile that card.

## Implementation: `night_terrors.rs`
- **Name:** Night Terrors -- CORRECT
- **Mana Cost:** {2}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None -- CORRECT
- **Target:** PlayerOnly -- CORRECT
- **oracle_text field:** "Target player reveals their hand. You choose a nonland card from it and exile that card." -- ISSUE: oracle has two separate sentences ("from it. Exile that card."), code merges them with "and" ("from it and exile that card.")
- **Behavior:** Auto-selects the first nonland card found via `hand.iter().find(...)` -- ISSUE: the oracle says "You choose a nonland card", meaning the controller of Night Terrors should be presented with a choice among all nonland cards in the target player's hand. The current implementation does not offer a choice; it just exiles the first one it finds.

## Verdict: ISSUE

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Target player reveals their hand. You choose a nonland card from it. Exile that card.
**Type line**: Sorcery
**Status**: ISSUE
### Code issues
1. **oracle_text mismatch**: Oracle says "You choose a nonland card from it. Exile that card." but code has "You choose a nonland card from it and exile that card." (two sentences merged into one).
2. **Missing player choice**: The `on_resolve` method uses `hand.iter().find(...)` to auto-select the first nonland card rather than presenting the caster with a choice among all nonland cards in the revealed hand. Per oracle text, the caster should choose which nonland card to exile.
