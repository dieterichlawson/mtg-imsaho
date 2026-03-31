# Audit: Angel of Flight Alabaster

## Reference (Scryfall/API)
- **Name:** Angel of Flight Alabaster
- **Mana Cost:** {4}{W}
- **Type:** Creature — Angel
- **Oracle:** Flying. At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
- **P/T:** 4/4

## Implementation: `angel_of_flight_alabaster.rs`
- **Name:** Angel of Flight Alabaster -- CORRECT
- **Mana Cost:** {4}{W} -- CORRECT
- **Type:** Creature — Angel -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Triggered ability:** Upkeep trigger, returns Spirit from graveyard to hand -- CORRECT
- **Target filtering:** Checks both registry subtypes and object subtypes for "Spirit" -- CORRECT

## Verdict: PASS -- No issues found
