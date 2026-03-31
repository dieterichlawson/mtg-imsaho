# Audit: Graveyard Shovel

## Oracle Reference (Scryfall)
- Cost: {2}
- Type: Artifact
- Oracle: "{2}, {T}: Exile target card from a graveyard. If it was a creature card, you gain 2 life."

## Implementation: graveyard_shovel.rs

## Issues Found

1. **ISSUE: Auto-targets instead of player choice** - Oracle says "target card from a graveyard" meaning the player should choose which card to exile. The implementation auto-selects, preferring creature cards for life gain (line 63-65). This removes strategic choice (e.g., choosing to exile a key non-creature card from opponent's graveyard even though creature cards exist).

Otherwise correct: cost ({2}), type (Artifact), oracle text, activated ability cost ({2} + tap), exile effect, and life gain for creature cards all match.

## Verdict: ISSUES FOUND (1 issue - auto-targeting)
