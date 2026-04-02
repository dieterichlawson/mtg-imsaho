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

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Defender, protection from Zombies
```

### Findings
- Name, cost ({1}{G}{G}), type (Creature -- Plant), P/T (3/4) all match.
- Defender keyword present in keywords vec -- correct.
- Protection from Zombies via ContinuousEffect::ProtectionFromSubtype with subtype "Zombie" and OnSelf scope -- correct.

### Verdict: PASS
