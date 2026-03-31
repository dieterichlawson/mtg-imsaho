# Audit: Bloodgift Demon

## Oracle (Scryfall)
- **Name:** Bloodgift Demon
- **Cost:** {3}{B}{B}
- **Type:** Creature — Demon
- **Oracle:** Flying. At the beginning of your upkeep, target player draws a card and loses 1 life.
- **P/T:** 5/4

## Implementation: `mtg-engine/src/cards/bloodgift_demon.rs`
- **Name:** Bloodgift Demon ✅
- **Cost:** {3}{B}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Demon ✅
- **P/T:** 5/4 ✅
- **Keywords:** Flying ✅
- **Triggered ability:** Upkeep ✅
- **on_upkeep:** Triggers only on controller's upkeep ✅
- **Target choice:** Presents all non-lost players as options via AwaitingAction ✅
- **Effect:** DrawAndLoseLife pending effect ✅
- **Zone check:** checks self is on battlefield ✅

## Verdict: PASS — no issues found
