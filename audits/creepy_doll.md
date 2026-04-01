## Audit — 2026-04-01

**Scryfall Oracle text**: Indestructible
Whenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Scryfall type line**: Artifact Creature — Construct
**Status**: ISSUE

### Findings

1. **Trigger implementation uses Blocks/BecomesBlocked instead of CombatDamageDealt (ISSUE)**: Oracle says "Whenever Creepy Doll deals combat damage to a creature." The implementation triggers on `TriggerKind::Blocks` and `TriggerKind::BecomesBlocked` (lines 33-38), which fire when blockers are declared, not when combat damage is dealt. This is incorrect — the trigger should fire during the combat damage step when Creepy Doll actually deals its damage. If Creepy Doll is destroyed before damage (e.g., by first strike), the trigger should not fire, but the current implementation would still fire it at blocker declaration.

2. **Card data correct**: Name, cost ({5}), types (Artifact, Creature), subtype (Construct), P/T (1/1), keyword (Indestructible) all match.

3. **Uses try_destroy**: Correct (line 60) — properly respects indestructible on the target.

4. **Coin flip logic**: Uses `gen_bool(0.5)` which is correct for a fair coin flip.

5. **Tests**: No dedicated tests found.
