# Audit: Bloodgift Demon

## Oracle Text (Scryfall)
- **Name:** Bloodgift Demon
- **Mana Cost:** {3}{B}{B}
- **Type:** Creature — Demon
- **P/T:** 5/4
- **Oracle Text:** Flying / At the beginning of your upkeep, target player draws a card and loses 1 life.

## Implementation File
`mtg-engine/src/cards/isd/bloodgift_demon.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({3}{B}{B})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Demon)
- **P/T:** Correct (5/4)
- **Keywords:** Correct (Flying)
- **Triggered ability:** Correctly registered as `Upkeep`

## Behavior Checks
- **on_upkeep:** Fires only on controller's upkeep (checks `active_player == controller`). Correct.
- **Target player choice:** Presents all non-lost players as targets via `AwaitingAction::ResolutionChoice`. Correct.
- **Effect:** `DrawAndLoseLife` pending effect -- delegates to engine to draw a card and lose 1 life. Correct.

## Verdict: PASS
