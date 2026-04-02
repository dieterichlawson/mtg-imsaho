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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
**Mana cost**: {3}{G}
**Type line**: Creature — Human Scout
**P/T**: 2/3
**Status**: ISSUE
### Code issues
1. **Oracle text string mismatch**: Oracle says `"Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand."` but code has `"Morbid — When Woodland Sleuth enters the battlefield, if a creature died this turn, return a creature card at random from your graveyard to your hand."`. The oracle template was updated to use "this creature enters" instead of the old "[Card Name] enters the battlefield" wording.
### Behavior
Behavior is correct: on_enter_battlefield checks creature_died_this_turn flag (morbid), finds creature cards in controller's graveyard, randomly selects one, and moves it to hand. Logic is sound.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "When this creature enters" (was "When Woodland Sleuth enters the battlefield"). Doc comment updated. Behavior unchanged.
