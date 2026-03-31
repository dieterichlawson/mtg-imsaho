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
