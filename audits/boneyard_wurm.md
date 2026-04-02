# Audit: Boneyard Wurm

## Oracle Text (Scryfall)
- **Name:** Boneyard Wurm
- **Mana Cost:** {1}{G}
- **Type:** Creature — Wurm
- **P/T:** */*
- **Oracle Text:** Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.

## Implementation File
`mtg-engine/src/cards/isd/boneyard_wurm.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({1}{G})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Wurm)
- **P/T:** Base set to (0, 0) with `dynamic_pt` override -- acceptable representation of */*.
- **Oracle Text:** Correct

## Behavior Checks
- **dynamic_pt:** Counts objects in controller's graveyard that have `power.is_some()` (i.e., creatures), returns that count as both power and toughness. Correct.

### Note (minor)
- Per Scryfall ruling: "The ability that defines Boneyard Wurm's power and toughness works in all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it will count itself." The `dynamic_pt` implementation does query by controller and zone correctly, but whether it functions in all zones depends on the engine calling `dynamic_pt` outside the battlefield. This is an engine-level concern, not a card implementation bug.

## Verdict: PASS
