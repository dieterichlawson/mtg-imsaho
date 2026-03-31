# Audit: Hamlet Captain

## Oracle Reference (Scryfall)
- Cost: {1}{G}
- Type: Creature -- Human Warrior
- P/T: 2/2
- Oracle: "Whenever Hamlet Captain attacks or blocks, other Human creatures you control get +1/+1 until end of turn."

## Implementation: hamlet_captain.rs

## Issues Found

No issues found. Name, cost ({1}{G}), type (Creature), subtypes (Human, Warrior), P/T (2/2), oracle text, and both triggered abilities (attacks, blocks) all match. The buff correctly targets other Human creatures you control (excluding self) and applies +1/+1 until end of turn via UntilEndOfTurnEffect.

## Verdict: PASS
