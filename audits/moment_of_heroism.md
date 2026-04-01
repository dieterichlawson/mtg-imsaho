## Audit — 2026-04-01

**Scryfall Oracle text**: Target creature gets +2/+2 and gains lifelink until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

- Name: Correct ("Moment of Heroism")
- Cost: {1}{W} - Correct
- Type: Instant - Correct
- Oracle text matches exactly.
- Target requirement: Creature - Correct (any creature, not just yours).
- On resolve: Applies +2/+2 via UntilEndOfTurnEffect and grants Lifelink via UntilEndOfTurnKeyword. Both correct.
- Checks target is still on battlefield before applying. Correct.
- Moves spell to graveyard after resolve. Correct.
- Tests: keywords.rs has lifelink-related test using Moment of Heroism.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Target creature gets +2/+2 and gains lifelink until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

No issues found. Correctly applies +2/+2 and lifelink until end of turn via UntilEndOfTurnEffect and UntilEndOfTurnKeyword. Uses move_spell_after_resolve. Targets a creature.
