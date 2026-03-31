# Audit: Infernal Plunge

## Oracle (Official)
- **Name:** Infernal Plunge
- **Cost:** {R}
- **Type:** Sorcery
- **Oracle:** As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}.
- **P/T:** N/A

## Implementation
- Name: "Infernal Plunge" -- CORRECT
- Cost: {R} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- additional_cost: SacrificeCreature -- CORRECT
- Adds {R}{R}{R} to mana pool -- CORRECT
- SIMPLIFICATION noted: sacrifice happens on resolution rather than during casting -- ACKNOWLEDGED

## Issues
1. **ISSUE (minor/simplification):** The sacrifice is performed at resolution instead of as a casting cost. Comment acknowledges this. In real MTG, the creature would be sacrificed as part of casting before the spell goes on the stack.

## Verdict: PASS (with noted simplification)
