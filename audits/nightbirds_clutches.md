## Audit — 2026-04-01

**Scryfall Oracle text**: Up to two target creatures can't block this turn.\nFlashback {3}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Correct ("Nightbird's Clutches")
- Cost: {1}{R} - Correct
- Type: Sorcery - Correct
- Oracle text matches.
- Flashback: {3}{R} - Correct
- Target: Up to two creatures (UpToTargets(2, Creature)) - Correct
- Effect: Adds target creatures to until_end_of_turn_cant_block. Correct.
- Tests: flashback.rs has `nightbirds_clutches_taps_creature`, card_mechanics.rs has `nightbirds_clutches_prevents_blocking`.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Up to two target creatures can't block this turn. Flashback {3}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. UpToTargets(2, Creature) targeting. Flashback {3}{R} correct. Applies cant_block via until_end_of_turn_cant_block. Uses move_spell_after_resolve.
