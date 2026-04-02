# Audit: Abattoir Ghoul

## Reference (Scryfall/API)
- **Name:** Abattoir Ghoul
- **Mana Cost:** {3}{B}
- **Type:** Creature — Zombie
- **Oracle:** First strike. Whenever a creature dealt damage by Abattoir Ghoul this turn dies, you gain life equal to that creature's toughness.
- **P/T:** 3/2

## Implementation: `abattoir_ghoul.rs`
- **Name:** Abattoir Ghoul -- CORRECT
- **Mana Cost:** {3}{B} -- CORRECT
- **Type:** Creature — Zombie -- CORRECT
- **P/T:** 3/2 -- CORRECT
- **Keywords:** FirstStrike -- CORRECT
- **Triggered ability:** AnyCreatureDies, checks `dead_damaged_by.contains(&self_id)` -- CORRECT
- **Life gain:** Uses `dead_toughness` (last-known information) -- CORRECT
- **NonCombatDamageDealt:** N/A (life gain, not damage)

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike\nWhenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Abattoir Ghoul", cost {3}{B}, 3/2, type Creature — Zombie, keywords [FirstStrike], triggered ability on AnyCreatureDies correctly checks `dead_damaged_by` and gains life equal to `dead_toughness`. Behavior is correct.
