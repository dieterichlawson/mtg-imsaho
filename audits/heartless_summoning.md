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

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
```

### Findings
- Name, cost ({1}{B}), type (Enchantment) all match.
- Oracle text in code matches Scryfall oracle.
- ContinuousEffect::ReduceCost with reduction 2 for CreatureSpells -- correct.
- ContinuousEffect::ModifyPT with -1/-1 for Global(CreatureFilter::You) -- correct.

### Verdict: PASS
