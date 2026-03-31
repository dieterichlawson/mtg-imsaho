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
