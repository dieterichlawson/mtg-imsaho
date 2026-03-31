# Audit: Gnaw to the Bone

## Oracle Reference (Scryfall)
- Cost: {2}{G}
- Type: Instant
- Oracle: "You gain 2 life for each creature card in your graveyard.
  Flashback {3}{G}"

## Implementation: gnaw_to_the_bone.rs

## Issues Found

No issues found. Name, cost ({2}{G}), type (Instant), oracle text, flashback cost ({3}{G}), and effect (gain 2 life per creature card in graveyard) all match. The implementation correctly counts creature cards in the controller's graveyard, excluding the spell itself (still on stack).

## Verdict: PASS
