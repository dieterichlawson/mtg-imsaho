# Audit: Bump in the Night

## Oracle (Scryfall/API)
- **Name:** Bump in the Night
- **Cost:** {B}
- **Type:** Sorcery
- **Oracle:** Target opponent loses 3 life. Flashback {5}{R}
- **P/T:** N/A

## Implementation: `bump_in_the_night.rs`
- **Name:** Bump in the Night -- CORRECT
- **Cost:** {B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {5}{R} -- CORRECT
- **Target:** PlayerOnly, validated to exclude self (opponent only) -- CORRECT
- **Effect:** Target opponent loses 3 life -- CORRECT
- **Life loss:** Directly modifies life and emits LifeChanged event -- CORRECT

## Issues
1. **ISSUE (minor):** This is life loss, not damage. The implementation correctly does NOT emit NonCombatDamageDealt, which is correct -- life loss is distinct from damage in MTG rules.

## Verdict: PASS -- No issues found
