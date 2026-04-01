## Audit — 2026-04-01

**Scryfall Oracle text**: When Crossway Vampire enters the battlefield, target creature can't block this turn.
**Scryfall type line**: Creature — Vampire
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({1}{R}{R}), type (Creature), subtype (Vampire), P/T (3/2) all match.

2. **ETB trigger correct**: Uses `TriggerKind::EntersBattlefield` and `on_enter_battlefield` hook.

3. **Target semantics correct**: Targets any creature (not "another"), which is correct per Oracle. The `present_target_choice` call is mandatory (`optional: false`), which is correct since Oracle doesn't say "you may."

4. **Effect**: Applies `PendingEffect::CantBlockThisTurn` which matches the Oracle text.

5. **Tests**: No dedicated tests found.
