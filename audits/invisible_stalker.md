# Audit: Invisible Stalker

## Oracle (Official)
- **Name:** Invisible Stalker
- **Cost:** {1}{U}
- **Type:** Creature — Human Rogue
- **Oracle:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) Invisible Stalker can't be blocked.
- **P/T:** 1/1

## Implementation
- Name: "Invisible Stalker" -- CORRECT
- Cost: {1}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Human", "Rogue"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Hexproof] -- CORRECT
- Continuous effects: CantBeBlocked { scope: OnSelf } -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Invisible Stalker
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Human Rogue
- **Cost:** {1}{U}
- **P/T:** 1/1
- **Oracle:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) / This creature can't be blocked.

### Card Data
- **Name:** Invisible Stalker -- PASS
- **Cost:** {1}{U} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Human, Rogue -- PASS
- **P/T:** 1/1 -- PASS

### Oracle Text Match
- Code oracle_text says "Invisible Stalker can't be blocked" vs oracle "This creature can't be blocked." Cosmetic only, no functional difference.
- PASS (minor wording variance)

### Behavior Audit
- **Hexproof:** Listed in keywords vec. -- PASS
- **Can't be blocked:** ContinuousEffect::CantBeBlocked with scope OnSelf. -- PASS

### Result: PASS
