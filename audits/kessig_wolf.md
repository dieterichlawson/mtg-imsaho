# Audit: Kessig Wolf

## Oracle (Official)
- **Name:** Kessig Wolf
- **Cost:** {2}{R}
- **Type:** Creature — Wolf
- **Oracle:** {1}{R}: Kessig Wolf gains first strike until end of turn.
- **P/T:** 3/1

## Implementation
- Name: "Kessig Wolf" -- CORRECT
- Cost: {2}{R} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Wolf"] -- CORRECT
- P/T: 3/1 -- CORRECT
- Oracle text matches -- CORRECT
- Activated ability: {1}{R} cost, no tap required, grants first strike until end of turn -- CORRECT
- Uses UntilEndOfTurnKeyword for first strike -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Kessig Wolf
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Wolf
- **Cost:** {2}{R}
- **P/T:** 3/1
- **Oracle:** {1}{R}: This creature gains first strike until end of turn.

### Card Data
- **Name:** Kessig Wolf -- PASS
- **Cost:** {2}{R} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Wolf -- PASS
- **P/T:** 3/1 -- PASS

### Oracle Text Match
- Code uses "Kessig Wolf gains first strike" vs oracle "This creature gains first strike". Cosmetic only.
- PASS (minor wording variance)

### Behavior Audit
- **Activated ability cost:** {1}{R}, no tap required. -- PASS
- **Activated ability effect:** Grants FirstStrike via UntilEndOfTurnKeyword. -- PASS
- **Restrictions:** No once_per_turn, no sorcery_speed_only (can activate multiple times at instant speed). -- PASS
- **Zone check:** Only available on battlefield. -- PASS

### Result: PASS
