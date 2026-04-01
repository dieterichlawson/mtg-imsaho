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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Trample / When Moldgraf Monstrosity dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Scryfall type line**: Creature — Insect
**Status**: PASS

Mana cost {4}{G}{G}{G}: correct. Type Creature, subtype Insect: correct. P/T 8/8: correct. Keyword Trample: correct. Triggered ability SelfDies declared: correct.

on_dies behavior: exiles self via `move_object(object_id, Zone::Exile)` (correct, not moving to graveyard), then finds creature cards in controller's graveyard (excluding self since it's now exiled), shuffles them randomly using `rand::thread_rng()` and `SliceRandom::shuffle`, and takes up to 2. This is correct -- the previous audit's "not random" issue has been fixed; the code now imports `rand` and uses `.shuffle(&mut rng)`.

Creatures are moved to battlefield and controller is set: correct. Tests present in `tests/tier15_cards.rs`. No anti-patterns found.
