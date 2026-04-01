## Audit — 2026-04-01

**Scryfall Oracle text**: Creatures you control get +2/+0 until end of turn.\nFlashback {2}{R}
**Scryfall type line**: Instant
**Status**: PASS

- Name: Correct ("Rally the Peasants")
- Cost: {2}{W} - Correct
- Type: Instant - Correct
- Flashback: {2}{R} - Correct
- Oracle text matches.
- on_resolve: Finds all creatures controlled by the caster on the battlefield, applies +2/+0 via UntilEndOfTurnEffect to each. Correct.
- Note: Only affects creatures on the battlefield at the time of resolution (snapshot), which is correct MTG behavior for this effect.
- Tests: innistrad_cards.rs has `rally_the_peasants_buffs_all_your_creatures`.

No issues found.
