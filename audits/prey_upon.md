## Audit — 2026-04-01

**Scryfall Oracle text**: Target creature you control fights target creature you don't control.
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Correct ("Prey Upon")
- Cost: {G} - Correct
- Type: Sorcery - Correct
- Oracle text matches.
- Target requirement: TwoTargets -- one creature you control, one creature you don't control. Correct.
- on_resolve: Identifies which target is yours and which is theirs, then calls combat::fight. Handles both orderings of targets. Correct.
- Tests: tier2_spells.rs has `prey_upon_fight`.

No issues found.
