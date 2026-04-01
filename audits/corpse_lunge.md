## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Scryfall type line**: Instant
**Status**: ISSUE

### Findings

1. **Additional cost handled at resolve time, not cast time (ISSUE)**: The `additional_cost` field is set to `AdditionalCost::ExileCreaturesFromGraveyard(1)` (line 29), which should handle exiling at cast time. However, the `on_resolve` method (line 43-46) also searches the graveyard for a creature to exile. This means the exile might happen twice, or the on_resolve exile is redundant/conflicting with the additional_cost mechanism. The power reference should come from the card exiled as part of the additional cost, not from a new search at resolve time.

2. **Exile target selection is not player-chosen (ISSUE)**: The implementation auto-selects the highest-power creature (line 45: `max_by_key`). Oracle says the player chooses which creature card to exile as an additional cost.

3. **Card data correct**: Name, cost ({2}{B}), type (Instant) match.

4. **NonCombatDamageDealt event correct**: Correctly uses `NonCombatDamageDealt` (line 61).

5. **Uses move_spell_after_resolve**: Correct (line 72).

6. **Tests**: No dedicated tests found.
