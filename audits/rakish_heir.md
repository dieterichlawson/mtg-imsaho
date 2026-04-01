## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire
**Status**: PASS

- Name: Correct ("Rakish Heir")
- Cost: {2}{R} - Correct
- Type: Creature — Vampire - Correct
- P/T: 2/2 - Correct
- Trigger: AnyCombatDamageToPlayer - Correct
- Implementation: Checks if the source creature is a Vampire controlled by Rakish Heir's controller, then puts +1/+1 counter on the source (the Vampire that dealt damage). Correct.
- Checks both registry card_data subtypes and runtime subtypes for Vampire. Correct (handles Olivia's type-changing ability).
- Tests: tier6_cards.rs has `rakish_heir_self_counter_on_combat_damage`, `rakish_heir_counter_on_other_vampire_combat_damage`, and `rakish_heir_no_counter_on_non_vampire`. Good coverage.

Note: Oracle says "put a +1/+1 counter on it" (referring to the Vampire), implementation puts counter on source_id. Correct.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire
**Status**: PASS

No issues found. Correctly checks both registry subtypes and obj.subtypes for Vampire type (handles tokens). Puts counter on the Vampire that dealt damage, not on Rakish Heir itself.
