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

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
```

### Findings
- Name, cost ({1}{G}), type (Creature -- Human Warrior), P/T (2/2) all match.
- Triggered abilities for Attacks and Blocks both call buff_humans -- correct.
- buff_humans correctly finds other Human creatures controlled by same player and applies +1/+1 until end of turn -- correct.
- Correctly excludes self (o.id != self_id) -- correct.

### ISSUE: Oracle text mismatch in code
- **Oracle (Scryfall)**: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn."
- **Code oracle_text**: "Whenever Hamlet Captain attacks or blocks, other Human creatures you control get +1/+1 until end of turn."

Two differences: (1) "Hamlet Captain" vs "this creature", (2) "Human creatures" vs "Humans". Behavior is functionally correct regardless.

### Verdict: ISSUE
Oracle text in code does not match current Scryfall oracle wording.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn." (was "Hamlet Captain attacks or blocks, other Human creatures"). Doc comment updated. Behavior unchanged.
