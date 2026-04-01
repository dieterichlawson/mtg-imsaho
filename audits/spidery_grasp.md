## Audit — 2026-04-01

**Scryfall Oracle text**: Untap target creature. It gets +2/+4 and gains reach until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

- Name: correct ("Spidery Grasp")
- Cost: {2}{G} -- correct
- Type: Instant -- correct
- Oracle text: matches
- Target: TargetRequirement::Creature -- correct
- Implementation untaps the creature, applies +2/+4 via UntilEndOfTurnEffect, and grants Reach via UntilEndOfTurnKeyword
- Correctly validates target is still on battlefield before applying effects
- Tests exist in `innistrad_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Untap target creature. It gets +2/+4 and gains reach until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

No issues found. Correctly untaps, applies +2/+4 until end of turn, grants reach until end of turn, uses `move_spell_after_resolve`.
