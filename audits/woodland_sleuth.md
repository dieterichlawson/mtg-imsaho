# Audit: Woodland Sleuth

## Scryfall Reference
- **Name:** Woodland Sleuth
- **Cost:** {3}{G}
- **Type:** Creature — Human Scout
- **Oracle:** Morbid -- When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
- **P/T:** 2/3

## Implementation: `mtg-engine/src/cards/woodland_sleuth.rs`
- Name: "Woodland Sleuth" -- MATCH
- Cost: {3}{G} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Scout"] -- MATCH
- P/T: 2/3 -- MATCH
- Trigger: EntersBattlefield -- MATCH
- Morbid check: uses state.creature_died_this_turn -- MATCH
- Behavior: Finds creature cards in controller's graveyard, shuffles them, returns one at random to hand -- MATCH

## Verdict
**PASS** — Morbid ETB correctly implemented with random selection.
