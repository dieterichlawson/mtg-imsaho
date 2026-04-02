# Audit: Gruesome Deformity

## Oracle Reference (Scryfall)
- Cost: {B}
- Type: Enchantment -- Aura
- Oracle: "Enchant creature
  Enchanted creature has intimidate."

## Implementation: gruesome_deformity.rs

## Issues Found

No issues found. Name, cost ({B}), type (Enchantment), subtype (Aura), oracle text, target requirement (Creature), and continuous effect (GrantKeyword Intimidate with EffectScope::Attached) all match.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

### Findings
- Name, cost ({B}), type (Enchantment -- Aura) all match.
- Oracle text in code: "Enchanted creature has intimidate." -- correct (reminder text omission is standard).
- Target requirement: Creature -- correct.
- Grants Intimidate via ContinuousEffect::GrantKeyword to Attached scope -- correct.
- Resolves via resolve_aura helper -- correct.

### Verdict: PASS
