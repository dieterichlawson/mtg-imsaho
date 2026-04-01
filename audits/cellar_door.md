## Audit — 2026-04-01

**Scryfall Oracle text**: {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
**Scryfall type line**: Artifact
**Status**: ISSUE

### Findings

1. **Oracle text field uses "mills" terminology (minor ISSUE)**: The oracle_text field says "mills the bottom card" but the actual Oracle text does not use the "mill" keyword (since mill specifically means from the top). The correct Oracle wording is "puts the bottom card of their library into their graveyard." This doesn't affect functionality but the oracle_text string is inaccurate.

2. **Bottom-of-library behavior correct**: Implementation correctly takes the last element of `library_order` (line 68), which represents the bottom card.

3. **Token creator correct**: The token is created under `controller` (the Cellar Door's controller), not the target player. This matches "you create" in the Oracle text.

4. **Card data correct**: Name, cost ({2}), type (Artifact) all match.

5. **Token correct**: 2/2 black Zombie creature token with proper subtypes.

6. **Tests**: No dedicated tests found.
