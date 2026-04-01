## Audit — 2026-04-01

**Scryfall Oracle text**: Return target creature card from your graveyard to the battlefield.\nFlashback {3}{W}
**Scryfall type line**: Sorcery
**Scryfall mana cost**: {4}{B}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {4}{B} — correct (Generic(4), Black).
- Type: Sorcery — correct.
- Oracle text: Matches.
- Flashback cost: {3}{W} — correct (Generic(3), White).
- Resolution: Finds creature cards in controller's graveyard, moves to battlefield. Handles single target auto-selection and multi-target player choice. Correct.
- Targeting: Uses `o.power.is_some()` as creature heuristic, which is reasonable.
- Tests: `unburial_rites_returns_creature` in flashback.rs, `unburial_rites_choice_with_multiple_creatures` in card_mechanics.rs.

No issues found.
