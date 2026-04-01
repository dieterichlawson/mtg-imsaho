## Audit — 2026-04-01

**Scryfall Oracle text**: {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
**Scryfall type line**: Creature — Homunculus
**Status**: ISSUE

- Name: correct ("Stitcher's Apprentice")
- Cost: {1}{U} -- correct
- Type: Creature -- correct
- Subtypes: Homunculus -- correct
- P/T: 1/2 -- correct
- Activated ability cost: {1}{U}, {T} -- correct
- Token: 2/2 blue Homunculus creature token -- correct

**Issue: Sacrifice choice is automated incorrectly.** The Oracle text says "sacrifice a creature" -- the controller should choose which creature to sacrifice. The implementation auto-selects, preferring non-token creatures over tokens. This is backwards from what a player would typically want (they'd usually sacrifice the token they just created). The sacrifice should be a player choice. Additionally, this is a "you" sacrifice (the controller sacrifices), which is correct, but the auto-choice heuristic is wrong.

- Tests exist in `tier8_cards.rs`
