## Audit — 2026-04-01

**Scryfall Oracle text**: Night Revelers has haste as long as an opponent controls a Human.
**Scryfall type line**: Creature — Vampire
**Status**: PASS

- Name: Correct ("Night Revelers")
- Cost: {4}{R} - Correct
- Type: Creature — Vampire - Correct
- P/T: 4/4 - Correct
- Conditional keyword: Haste when opponent controls a Human subtype creature. Implemented via ContinuousEffect::ConditionalKeyword with OpponentControlsSubtype("Human") and OnSelf scope. Correct.
- Tests: tier12_cards.rs has `night_revelers_has_haste_with_opponent_human` which tests gaining and losing haste dynamically.

No issues found.
