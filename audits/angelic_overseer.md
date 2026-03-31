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
