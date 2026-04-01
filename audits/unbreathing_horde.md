## Audit — 2026-04-01

**Scryfall Oracle text**: Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.\nIf Unbreathing Horde would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
**Scryfall type line**: Creature — Zombie
**Scryfall mana cost**: {2}{B}
**Scryfall P/T**: 0/0
**Status**: ISSUE

Findings:
- Name: Correct.
- Mana cost: {2}{B} — correct.
- Types: Creature — Zombie — correct.
- P/T: 0/0 — correct.
- ETB counter logic: Correctly counts other Zombies on battlefield and Zombie cards in graveyard, adds +1/+1 counters. Correct.
- **ISSUE: Damage prevention/replacement not properly implemented.** The Oracle text says "If Unbreathing Horde would be dealt damage, prevent that damage and remove a +1/+1 counter from it." This is a damage replacement effect. The implementation comment acknowledges this is approximate — it does not actually implement the damage prevention or the counter removal on damage. The card just gets counters on ETB. Without the damage-to-counter-removal replacement, the card will die to normal damage like any other creature instead of losing counters. This is a significant functional gap.
- Tests: `unbreathing_horde_enters_with_counters_for_zombies` tests ETB counters only, not the damage replacement.

## Audit — 2026-04-01

**Scryfall Oracle text**: Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard. If Unbreathing Horde would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
**Scryfall type line**: Creature — Zombie
**P/T**: 0/0, **Mana cost**: {2}{B}
**Status**: ISSUE

1. **Damage prevention not fully implemented** (`mtg-engine/src/cards/unbreathing_horde.rs`): The card's damage prevention replacement effect (prevent damage, remove a counter) is not implemented. The code comment acknowledges this limitation. The ETB counter placement is correct.
