# Audit: Heartless Summoning

## Oracle Reference (Scryfall)
- Cost: {1}{B}
- Type: Enchantment
- Oracle: "Creature spells you cast cost {2} less to cast.
  Creatures you control get -1/-1."

## Implementation: heartless_summoning.rs

## Issues Found

No issues found. Name, cost ({1}{B}), type (Enchantment), oracle text, and both continuous effects (ReduceCost for creature spells by 2, ModifyPT -1/-1 for creatures you control) all match correctly.

## Verdict: PASS
