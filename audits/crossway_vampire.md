# Audit: Crossway Vampire

## Scryfall Reference
- **Name:** Crossway Vampire
- **Cost:** {1}{R}{R}
- **Type:** Creature -- Vampire
- **Oracle:** When this creature enters, target creature can't block this turn.
- **P/T:** 3/2
- **Keywords:** none

## Implementation: `crossway_vampire.rs`
- **Name:** Crossway Vampire -- CORRECT
- **Cost:** {1}{R}{R} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Vampire"] -- CORRECT
- **P/T:** 3/2 -- CORRECT
- **Keywords:** none -- CORRECT
- **Trigger:** EntersBattlefield -- CORRECT
- **Behavior:** Presents target choice for "can't block this turn" -- CORRECT

## Issues
None
