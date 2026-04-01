## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.
**Scryfall type line**: Creature — Human Shaman
**Status**: ISSUE

- Name: Correct ("Rage Thrower")
- Cost: {5}{R} - Correct
- Type: Creature — Human Shaman - Correct
- P/T: 4/2 - Correct
- Trigger: AnyCreatureDies - Correct ("another creature")

Issues:
1. **Target restriction too narrow**: Oracle says "target player or planeswalker" but the implementation only offers players as targets (no planeswalkers). While planeswalkers may not exist in this Innistrad-only engine, the targeting semantics are slightly off.

- Tests: tier3_cards.rs has `rage_thrower_deals_2_on_death`. apnap.rs uses Rage Thrower for APNAP ordering tests.
## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.
**Scryfall type line**: Creature — Human Shaman
**Status**: ISSUE

- **Missing planeswalker targeting**: Oracle says "target player or planeswalker" but implementation at `mtg-engine/src/cards/rage_thrower.rs:44` only offers `Target::Player` options, not planeswalkers. Minor since planeswalkers may not be in the card pool, but technically incomplete.
