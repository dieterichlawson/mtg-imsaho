## Audit — 2026-04-01

**Scryfall Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G}
**Scryfall type line**: Sorcery
**Status**: ISSUE

### Findings

1. **Type choice is hardcoded to "creature" (ISSUE)**: Oracle says "Choose a permanent type" — the player should pick from creature, artifact, enchantment, land, or planeswalker. The implementation always chooses "creature" (line 46-53). While creature is commonly the best choice, this removes player agency and is incorrect for cases where returning artifacts, enchantments, or lands would be better.

2. **Card data correct**: Name, cost ({3}{G}{G}), type (Sorcery) match.

3. **Flashback cost correct**: {5}{G}{G} matches Oracle.

4. **Uses move_spell_after_resolve**: Correct (line 62).

5. **Tests**: No dedicated tests found.
