# Audit: Champion of the Parish

## Scryfall Reference
- **Name:** Champion of the Parish
- **Cost:** {W}
- **Type:** Creature -- Human Soldier
- **Oracle:** Whenever another Human you control enters, put a +1/+1 counter on this creature.
- **P/T:** 1/1
- **Keywords:** none

## Implementation: `champion_of_the_parish.rs`
- **Name:** Champion of the Parish -- CORRECT
- **Cost:** {W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Human", "Soldier"] -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** none -- CORRECT
- **Trigger:** AnyCreatureEnters -- CORRECT
- **Behavior:** Checks Human subtype, checks controller match, adds +1/+1 counter -- CORRECT

## Issues
None
