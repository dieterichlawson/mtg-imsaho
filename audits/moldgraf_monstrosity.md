# Audit: Moldgraf Monstrosity

## Official Oracle
- **Name:** Moldgraf Monstrosity
- **Cost:** {4}{G}{G}{G}
- **Type:** Creature — Insect
- **Oracle:** Trample. When Moldgraf Monstrosity dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
- **P/T:** 8/8

## Implementation: `mtg-engine/src/cards/moldgraf_monstrosity.rs`
- **Name:** Moldgraf Monstrosity -- CORRECT
- **Cost:** {4}{G}{G}{G} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Insect -- CORRECT
- **P/T:** 8/8 -- CORRECT
- **Keywords:** Trample -- CORRECT
- **Triggered ability:** SelfDies -- CORRECT
- **on_dies:** Exiles self, returns up to 2 creatures from graveyard -- CORRECT

## Issues
1. **Not random:** Oracle says "at random" but implementation uses `.take(2)` (first 2 found) instead of random selection. Comment in code acknowledges this: "Use a simple deterministic selection (first 2) since we don't have rng here." However, the file does not import `rand` even though other cards in the codebase do.

## Verdict
**FAIL** -- 1 issue: Creature selection is deterministic (first 2) instead of random.
