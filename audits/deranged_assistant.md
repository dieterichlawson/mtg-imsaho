## Audit — 2026-04-01

**Scryfall Oracle text**: {T}, Mill a card: Add {C}.
**Scryfall type line**: Creature — Human Wizard
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({1}{U}), type (Creature), subtypes (Human, Wizard), P/T (1/1) all match.

2. **Mana ability correct**: Produces {C} (colorless), requires tap, mills a card as additional cost.

3. **Mill cost handled correctly**: `on_activate_mana_ability` calls `mill_cards(state, controller, 1)`.

4. **Library check**: Correctly prevents activation when library is empty (line 40).

5. **Summoning sickness check**: Correctly checks `!obj.summoning_sick` (line 41) since this is a creature with a tap ability.

6. **Tests**: No dedicated tests found.
