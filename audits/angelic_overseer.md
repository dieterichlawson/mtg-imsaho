# Audit: Angelic Overseer

## Reference (Scryfall/API)
- **Name:** Angelic Overseer
- **Mana Cost:** {3}{W}{W}
- **Type:** Creature — Angel
- **Oracle:** Flying. As long as you control a Human, Angelic Overseer has hexproof and indestructible.
- **P/T:** 5/3

## Implementation: `angelic_overseer.rs`
- **Name:** Angelic Overseer -- CORRECT
- **Mana Cost:** {3}{W}{W} -- CORRECT
- **Type:** Creature — Angel -- CORRECT
- **P/T:** 5/3 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Continuous effects:** ConditionalKeyword Hexproof (YouControlSubtype Human) + ConditionalKeyword Indestructible (YouControlSubtype Human) -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nAs long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"As long as you control a Human, this creature has hexproof and indestructible."` but code oracle_text field says `"As long as you control a Human, Angelic Overseer has hexproof and is indestructible."` The code uses the old self-referential template instead of the updated "this creature" template.
   - Code: `"As long as you control a Human, Angelic Overseer has hexproof and is indestructible."`
   - Oracle: `"As long as you control a Human, this creature has hexproof and indestructible."`

Behavior is otherwise correct: two ConditionalKeyword continuous effects (Hexproof, Indestructible) both conditioned on YouControlSubtype("Human") with OnSelf scope. Stats, cost, types, and keywords all match.
