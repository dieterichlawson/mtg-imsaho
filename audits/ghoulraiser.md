# Audit: Ghoulraiser

## Oracle Reference (Scryfall)
- Cost: {1}{B}{B}
- Type: Creature -- Zombie
- P/T: 2/2
- Oracle: "When Ghoulraiser enters the battlefield, return a Zombie creature card at random from your graveyard to your hand."

NOTE: Current Scryfall oracle errata says "Zombie card" not "Zombie creature card". Original printing said "Zombie creature card".

## Implementation: ghoulraiser.rs

## Issues Found

1. **BUG (from prior audit): Filters for "Zombie creature card" instead of "Zombie card"** - Engine code (line 51-53) filters for is_creature && is_zombie, but updated oracle only requires Zombie subtype. Low severity since all Zombies in the set are creatures.

Otherwise correct: cost ({1}{B}{B}), type (Creature), subtype (Zombie), P/T (2/2), ETB trigger, random selection.

## Verdict: ISSUES FOUND (1 minor issue)
