# Audit: Memory's Journey

## Official Oracle
- **Name:** Memory's Journey
- **Cost:** {1}{U}
- **Type:** Instant
- **Oracle:** Target player shuffles up to three target cards from their graveyard into their library. Flashback {G}

## Implementation: `mtg-engine/src/cards/memorys_journey.rs`
- **Name:** Memory's Journey -- CORRECT
- **Cost:** {1}{U} -- CORRECT
- **Type:** Instant -- CORRECT
- **Flashback:** {G} -- CORRECT
- **Target:** PlayerOnly -- PARTIAL (see below)
- **on_resolve:** Moves up to 3 graveyard cards to library, shuffles -- CORRECT behavior

## Issues
1. **Targeting simplified:** The oracle says "Target player shuffles up to three **target** cards from their graveyard." The cards in the graveyard are also targets, not just the player. The implementation auto-picks the first 3 cards from the graveyard using `.take(3)` rather than targeting specific cards. This is a simplification -- the caster should choose which cards to shuffle back.

## Verdict
**FAIL** -- 1 issue: Graveyard card selection is auto-picked (first 3) instead of targeted.
