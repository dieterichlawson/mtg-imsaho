## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.\n{3}{B}{R}, {T}: Stensia Bloodhall deals 2 damage to target player or planeswalker.
**Scryfall type line**: Land
**Status**: ISSUE

- Name: correct ("Stensia Bloodhall")
- Cost: None (land) -- correct
- Type: Land -- correct
- Mana ability: {T}: Add {C} -- correct
- Activated ability cost: {3}{B}{R}, {T} -- correct
- Damage amount: 2 -- correct

**Issue: Target restriction is too narrow.** The Oracle text says "target player or planeswalker" but the implementation uses `TargetRequirement::PlayerOnly`, which means it cannot target planeswalkers. The oracle_text field in the implementation correctly includes "or planeswalker" but the actual targeting code does not support it. While the engine may not support planeswalkers, the oracle text in the implementation matches Scryfall, and this is likely an engine limitation rather than a card implementation bug.

- Tests exist in `tier10_cards.rs`
- Minor issue: damage is applied by directly modifying life rather than using a unified damage pipeline, which could miss damage triggers (e.g., Rage Thrower). However this is consistent with other cards in the engine.
