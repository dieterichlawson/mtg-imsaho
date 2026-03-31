# Audit: Grimoire of the Dead

## Oracle Reference (Scryfall)
- Cost: {4}
- Type: Legendary Artifact
- Oracle: "{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
  {T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types."

## Implementation: grimoire_of_the_dead.rs

## Issues Found

1. **ISSUE: Study counters tracked via card_state instead of CounterType** - The implementation uses `card_state.insert("study_counters", ObjectId(...))` (line 116) as a hacky way to store the counter count. This means other systems that interact with counters (e.g., proliferate, counter removal effects) won't see these counters. Should use a proper CounterType.

2. **ISSUE: Discard auto-selects card** - The discard cost should let the player choose which card to discard. The implementation auto-picks the first card in hand (line 100).

3. **ISSUE: Ability 0 shows even when tapped** - The activated_abilities check (line 47) verifies zone == Battlefield but the requires_tap field handles the tap requirement. However, the ability shouldn't appear as available if the artifact is already tapped. Looking more closely, the check is at line 47 which only checks zone, not tapped state. The requires_tap field should handle this at the engine level, so this may be fine depending on how the engine filters.

Otherwise correct: cost ({4}), type (Legendary Artifact), oracle text matches, sacrifice ability correctly removes counters and returns all graveyard creatures as black Zombies.

## Verdict: ISSUES FOUND (2 issues)
