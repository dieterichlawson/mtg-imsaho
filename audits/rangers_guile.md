## Audit — 2026-04-01

**Scryfall Oracle text**: Target creature you control gets +1/+1 and gains hexproof until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

- Name: Correct ("Ranger's Guile")
- Cost: {G} - Correct
- Type: Instant - Correct
- Oracle text matches.
- Target: Creature you control (CreatureWithFilter(YouControl)) - Correct
- is_valid_target: Checks zone == Battlefield, is a creature (power.is_some()), and controller == caster. Correct.
- on_resolve: Applies +1/+1 via UntilEndOfTurnEffect and grants Hexproof via UntilEndOfTurnKeyword. Correct.
- Tests: card_fixes.rs has `rangers_guile_cannot_target_opponent_creature`, innistrad_cards.rs has `rangers_guile_gives_hexproof_and_pump`.

No issues found.
