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
