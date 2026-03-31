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
