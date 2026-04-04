## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creature tokens you control get +1/+1 and have vigilance.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **CreatureFilter::YourTokens correctly identifies tokens**: `matches_filter` in `state.rs:653` checks `creature.controller == source_controller && creature.is_token`. This correctly limits the effect to tokens controlled by Intangible Virtue's controller. pass
- **Non-tokens excluded from +1/+1**: `continuous_pt_mods` applies the ModifyPT effect through `effect_applies_to`, which calls `matches_filter` with `CreatureFilter::YourTokens`. Non-token creatures (`is_token = false`) are excluded. pass
- **Non-tokens excluded from vigilance grant**: `has_keyword` in `state.rs:987` checks continuous `GrantKeyword` effects via `has_continuous_effect`. The same `YourTokens` filter excludes non-tokens from receiving vigilance. pass
- **Vigilance prevents tap on attack**: `combat::declare_attackers` in `combat.rs:19` calls `state.has_keyword(attacker_id, Keyword::Vigilance, registry)`, which checks continuous effect grants including `GrantKeyword` from Intangible Virtue. Tokens under Intangible Virtue will not tap when attacking. pass
- **Continuous re-evaluation (not a snapshot)**: `effective_power`, `effective_toughness`, and `has_keyword` each re-query all battlefield objects on every call. If Intangible Virtue leaves the battlefield, its effects immediately cease to apply. pass
- **Opponent's tokens not buffed**: `CreatureFilter::YourTokens` checks `creature.controller == source_controller`, so tokens controlled by the opponent are not affected. pass
- **Effect correctly scoped to battlefield**: `continuous_pt_mods` in `state.rs:730` skips any source object whose `zone != Zone::Battlefield`, so if Intangible Virtue is in the graveyard or hand it has no effect. pass
- **EffectScope::Global vs GlobalOther**: The card uses `EffectScope::Global`, which does not exclude the source itself. The source is an enchantment (no power/toughness), so this distinction is irrelevant, but the scope is correct as it doesn't need to be `GlobalOther`. pass
- **Multiple Intangible Virtues stack**: Since `continuous_pt_mods` iterates all battlefield objects independently, two copies would give tokens +2/+2 total (not tested, but the loop structure is additive). pass

### Test coverage
- Token gets +1/+1 from Intangible Virtue: `tier3_cards.rs:511-514` and `card_mechanics.rs:351-354`
- Token gets vigilance from Intangible Virtue: `tier3_cards.rs:516-517` and `card_mechanics.rs:355-356`
- Non-tokens not buffed: `tier3_cards.rs:519-525` and `card_mechanics.rs:358-363`
- Token does not tap when attacking due to Intangible Virtue-granted vigilance: NOT TESTED (keywords.rs:78 tests innate vigilance only, not granted vigilance)
- Effect ceases when Intangible Virtue leaves battlefield: NOT TESTED
- Opponent's tokens not buffed: NOT TESTED
- Multiple Intangible Virtues stacking: NOT TESTED
