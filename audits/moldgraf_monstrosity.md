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

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Trample. When Moldgraf Monstrosity dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Type line**: Creature — Insect
**Status**: PASS

Mana cost {4}{G}{G}{G}: correct (Generic(4) + 3x Green). Type Creature, subtype Insect: correct. P/T 8/8: correct. Keyword Trample: correct. Triggered ability `SelfDies` declared: correct.

`on_dies` behavior: (1) Gets controller from owner field. (2) Exiles self via `move_object(object_id, Zone::Exile)`: correct. (3) Finds creature cards in controller's graveyard, excluding self (already exiled): correct. (4) Shuffles candidates using `rand::thread_rng()` and `SliceRandom::shuffle`, takes up to 2: correct random selection matching "at random" oracle text. (5) Moves selected creatures to battlefield and sets controller: correct. Per Scryfall ruling, if Moldgraf Monstrosity can't be exiled (not in graveyard), the two creatures are still returned -- the code does `move_object` unconditionally which handles this. Per ruling, the ability has no targets (selects at random on resolution): correct, the `on_dies` handler doesn't use targets.

Tests in `tests/tier15_cards.rs` cover: basic death trigger with two graveyard creatures returned. No anti-patterns found (uses `move_object` to Exile for self, which is correct since this is a triggered ability moving the card, not a spell resolving).

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Trample / When Moldgraf Monstrosity dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Type line**: Creature — Insect
**Status**: PASS

Mana cost {4}{G}{G}{G}: correct. Type Creature, subtype Insect: correct. P/T 8/8: correct. Keyword Trample: correct. Triggered ability SelfDies declared: correct.

on_dies behavior: Exiles self via `move_object(object_id, Zone::Exile)` -- correct (it needs to exile from graveyard, not stay there). Finds creature cards in controller's graveyard (filtered by `power.is_some()` and excluding self since it's exiled). Shuffles randomly using `rand::thread_rng()` + `SliceRandom::shuffle`, takes up to 2: correct random selection per oracle. Moves selected creatures to battlefield and sets controller: correct.

Per Scryfall ruling: if Moldgraf can't exile itself, the two creatures are still returned. The code does the exile first and then finds creatures -- if the exile failed (e.g., already moved), the creature search and return would still execute, which is correct behavior per the ruling.

Per ruling: the ability does not target (selects at random on resolution): correct -- the code has no targeting, just random selection at resolution time.

Tests in `tests/tier15_cards.rs`: verifies exile and return of 2 creatures. No anti-patterns found.
