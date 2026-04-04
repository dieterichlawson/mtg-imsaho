# Audit: Lava Axe

## Oracle (Official)
- **Name:** Lava Axe
- **Cost:** {4}{R}
- **Type:** Sorcery
- **Oracle:** Lava Axe deals 5 damage to target player or planeswalker.
- **P/T:** N/A

## Implementation
- Name: "Lava Axe" -- CORRECT
- Cost: {4}{R} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- Target requirement: PlayerOnly -- POTENTIAL ISSUE (see below)
- Deals 5 damage via resolve_damage helper -- CORRECT

## Issues
1. **ISSUE (minor):** Target requirement is `PlayerOnly` but oracle says "target player or planeswalker." The card should also be able to target planeswalkers. This matters if the engine supports planeswalkers as targets.

## Verdict: PASS (minor targeting limitation for planeswalkers)
