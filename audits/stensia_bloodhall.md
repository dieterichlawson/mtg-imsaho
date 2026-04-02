# Audit: Stensia Bloodhall

## Oracle (Scryfall)
- **Name:** Stensia Bloodhall
- **Cost:** N/A (Land)
- **Type:** Land
- **Oracle:** {T}: Add {C}. {3}{B}{R}, {T}: Stensia Bloodhall deals 2 damage to target player or planeswalker.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/stensia_bloodhall.rs`
- **Name:** Stensia Bloodhall ✅
- **Cost:** None ✅
- **Type:** Land ✅
- **Mana ability:** {T}: Add {C} ✅
- **Activated ability:** {3}{B}{R}, {T}: deals 2 damage to target player ✅
- **Target:** PlayerOnly ✅
- **Damage events:** NonCombatDamageDealt + LifeChanged ✅

### Note
- Oracle says "target player or planeswalker" but implementation only targets players (PlayerOnly). Since planeswalkers are not in the card pool, this is acceptable.

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {T}: Add {C}.\n{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
**Mana cost**: None (Land)
**Type line**: Land
**Status**: ISSUE
### Checks
- **Name**: "Stensia Bloodhall" -- CORRECT
- **Mana cost**: None -- CORRECT (Land)
- **Type**: Land -- CORRECT
- **Mana ability**: Tap for {C} -- CORRECT
- **Activated ability cost**: {3}{B}{R}, tap -- CORRECT
- **Damage amount**: 2 -- CORRECT
- **Oracle text in code**: Matches oracle -- CORRECT
### Code issues
1. **ISSUE — Target restriction too narrow**: Oracle says "target player or planeswalker" but the code uses `TargetRequirement::PlayerOnly` and the `on_activate_ability` handler only matches `Target::Player`. Planeswalker targeting is missing.
   - Code: `target_requirement: Some(TargetRequirement::PlayerOnly)`
   - Oracle: "target player or planeswalker"
