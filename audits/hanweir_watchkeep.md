# Audit: Hanweir Watchkeep / Bane of Hanweir

## Oracle Reference (Scryfall)
**Front Face: Hanweir Watchkeep**
- Cost: {2}{R}
- Type: Creature -- Human Warrior Werewolf
- P/T: 1/5
- Oracle: "Defender
  At the beginning of each upkeep, if no spells were cast last turn, transform Hanweir Watchkeep."

**Back Face: Bane of Hanweir**
- Type: Creature -- Werewolf
- P/T: 5/5
- Oracle: "Bane of Hanweir attacks each combat if able.
  At the beginning of each upkeep, if a player cast two or more spells last turn, transform Bane of Hanweir."

## Implementation: hanweir_watchkeep.rs

## Issues Found

No issues found. Front and back face names, types, subtypes (Human Warrior Werewolf / Werewolf), P/T (1/5 / 5/5), defender keyword, forced attack on back face (ContinuousEffect::ForceAttack), and werewolf transform logic all match.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
**Front:**
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back (Bane of Hanweir):**
```
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

### Findings
- Name, cost ({2}{R}), type (Creature -- Human Warrior Werewolf), P/T (1/5 / 5/5) all match.
- Front face: Defender keyword present -- correct.
- Back face: Bane of Hanweir, Creature -- Werewolf, 5/5, ForceAttack effect -- correct.
- Transform logic matches standard werewolf pattern -- correct.

### ISSUE: Back face oracle text mismatch
- **Oracle (Scryfall)**: "This creature attacks each combat if able."
- **Code oracle_text**: "Bane of Hanweir attacks each combat if able."

The current Scryfall oracle uses "This creature" instead of the card name. Behavior is functionally correct regardless.

### Verdict: ISSUE
Back face oracle text in code uses "Bane of Hanweir" where current oracle uses "This creature".

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall for both faces: "transform this creature" (was "transform Hanweir Watchkeep"/"transform Bane of Hanweir"), and "This creature attacks each combat if able" (was "Bane of Hanweir attacks each combat if able"). Behavior unchanged.
