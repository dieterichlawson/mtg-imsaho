## Audit — 2026-04-01

**Scryfall Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Morbid — You may put that card onto the battlefield instead of into your hand.
**Scryfall type line**: Sorcery
**Status**: ISSUE

### Findings

1. **Morbid is not optional (ISSUE)**: Oracle says "You *may* put that card onto the battlefield instead." The implementation auto-chooses battlefield when morbid is active (line 62). While the comment acknowledges this ("strictly better in almost all cases"), it removes player agency. The player should be given a choice. In edge cases (e.g., landfall triggers you want to avoid, or an opponent's Ankh of Mishra), putting it into hand could be preferred.

2. **Card data correct**: Name, mana cost ({G}), type (Sorcery), and oracle text are correct.

3. **Shuffle always happens**: Correctly shuffles even when no basic land is found (lines 78-80).

4. **Uses move_spell_after_resolve**: Correct (line 83).

5. **No self-exclusion issues**: The "another" vs "a" distinction is not relevant here.

6. **Tests**: No dedicated tests found.
