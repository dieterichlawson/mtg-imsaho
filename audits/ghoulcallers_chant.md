# Audit: Ghoulcaller's Chant

## Oracle Reference (Scryfall)
- Cost: {B}
- Type: Sorcery
- Oracle: "Choose one --
  * Return target creature card from your graveyard to your hand.
  * Return two target Zombie creature cards from your graveyard to your hand."

NOTE: Current Scryfall oracle text says "Zombie cards" not "Zombie creature cards" for mode 2. However the original Innistrad printing says "Zombie creature cards". The current oracle errata simplified it.

## Implementation: ghoulcallers_chant.rs

## Issues Found

1. **ISSUE: Mode selection is automated, not player-chosen** - The implementation auto-selects mode 2 (return two Zombies) whenever there are 2+ Zombies in graveyard, and falls back to mode 1 otherwise. Per Oracle, the player chooses which mode. A player might want to return a single non-Zombie creature even when Zombies are available.

2. **BUG (from prior audit): Oracle text says "Zombie creature cards" but current errata says "Zombie cards"** - The engine filters for creature AND Zombie (lines 43-49), but updated oracle only requires Zombie subtype. Low severity since all Zombies in the set are creatures.

Otherwise correct: cost ({B}), type (Sorcery), oracle text structure matches.

## Verdict: ISSUES FOUND (2 issues)
