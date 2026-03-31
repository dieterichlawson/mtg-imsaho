# Audit: Ghoulcaller's Bell

## Oracle Reference (Scryfall)
- Cost: {1}
- Type: Artifact
- Oracle: "{T}: Each player mills a card."

## Implementation: ghoulcallers_bell.rs

## Issues Found

No issues found. Name, cost ({1}), type (Artifact), oracle text, and activated ability all match. The tap ability correctly mills 1 card from each player using crate::engine::mill_cards.

## Verdict: PASS
