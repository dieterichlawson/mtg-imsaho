## Audit — 2026-04-01

**Scryfall Oracle text**: You gain 2 life for each creature card in your graveyard.
Flashback {3}{G}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {2}{G}: correct
- Card type Instant: correct
- Flashback cost {3}{G}: correct
- On resolve: counts creature cards in controller's graveyard (excluding self on stack), gains 2 life per creature: correct
- Life gain emits LifeChanged event: correct
- Tests exist in flashback.rs covering life gain

## Audit — 2026-04-01

**Scryfall Oracle text**: You gain 2 life for each creature card in your graveyard. / Flashback {2}{G}
**Scryfall type line**: Instant
**Status**: ISSUE

1. **CRITICAL: Wrong flashback cost**: The code sets flashback_cost to {3}{G} (ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green)])) but the correct flashback cost is {2}{G}. The flashback cost should be the same as the regular mana cost. (Line 26 in gnaw_to_the_bone.rs)
