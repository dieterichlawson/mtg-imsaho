## Audit — 2026-04-01

**Scryfall Oracle text**: {2}{G}: Darkthicket Wolf gets +2/+2 until end of turn. Activate only once each turn.
**Scryfall type line**: Creature — Wolf
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({1}{G}), type (Creature), subtype (Wolf), P/T (2/2) all match.

2. **Activated ability correct**: Cost {2}{G}, no tap required, `once_per_turn: true` matches "Activate only once each turn."

3. **Effect correct**: +2/+2 until end of turn via `UntilEndOfTurnEffect`.

4. **Tests**: No dedicated tests found.
