# Audit: Grizzled Outcasts / Krallenhorde Wantons

## Oracle Reference (Scryfall)
**Front Face: Grizzled Outcasts**
- Cost: {4}{G}
- Type: Creature -- Human Werewolf
- P/T: 4/4
- Oracle: "At the beginning of each upkeep, if no spells were cast last turn, transform Grizzled Outcasts."

**Back Face: Krallenhorde Wantons**
- Type: Creature -- Werewolf
- P/T: 7/7
- Oracle: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Krallenhorde Wantons."

## Implementation: grizzled_outcasts.rs

## Issues Found

No issues found. Front and back face names, types, subtypes, P/T values, and oracle text all match. Werewolf transform logic is correct (same pattern as other werewolves).

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
**Front:**
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back (Krallenhorde Wantons):**
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

### Findings
- Name, cost ({4}{G}), type (Creature -- Human Werewolf), P/T (4/4 / 7/7) all match.
- Back face: Krallenhorde Wantons, Creature -- Werewolf, 7/7 -- correct.
- Transform logic: front transforms when no spells cast last turn (and not first turn); back transforms when any player cast 2+ spells -- correct.
- on_upkeep correctly checks battlefield zone, toggles is_transformed, updates name -- correct.
- dynamic_pt returns (7,7) when transformed -- correct.

### Verdict: PASS
