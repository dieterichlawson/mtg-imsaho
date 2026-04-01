## Audit — 2026-04-01

**Scryfall Oracle text**: Indestructible
Whenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Scryfall type line**: Artifact Creature — Construct
**Status**: ISSUE

1. **Trigger timing is wrong** (`mtg-engine/src/cards/creepy_doll.rs`, lines 30-38, 42-48): The triggered abilities use `TriggerKind::Blocks` and `TriggerKind::BecomesBlocked`, which trigger at block declaration. Oracle says "Whenever Creepy Doll deals combat damage to a creature," which should trigger after combat damage is dealt, not when blocks are declared. This means the coin flip and potential destruction happen too early (before damage) rather than after damage resolution. The correct trigger would be something like `TriggerKind::CombatDamageDealt` or similar.
