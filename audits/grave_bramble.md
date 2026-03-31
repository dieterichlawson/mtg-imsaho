# Audit: Grave Bramble

## Oracle Reference (Scryfall)
- Cost: {1}{G}{G}
- Type: Creature -- Plant
- P/T: 3/4
- Oracle: "Defender, protection from Zombies"

## Implementation: grave_bramble.rs

## Issues Found

No issues found. Name, cost ({1}{G}{G}), type (Creature), subtype (Plant), P/T (3/4), defender keyword, and protection from Zombies (via ContinuousEffect::ProtectionFromSubtype) all match.

## Verdict: PASS
