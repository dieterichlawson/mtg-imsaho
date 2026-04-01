## Audit — 2026-04-01

**Scryfall Oracle text**: Flash\nWhen Village Bell-Ringer enters the battlefield, untap all creatures you control.
**Scryfall type line**: Creature — Human Scout
**Scryfall mana cost**: {2}{W}
**Scryfall P/T**: 1/4
**Status**: PASS

Findings:
- Name: Correct ("Village Bell-Ringer").
- Mana cost: {2}{W} — correct.
- Types: Creature — Human Scout — correct.
- P/T: 1/4 — correct.
- Keywords: Flash — correct.
- ETB: Untaps all creatures controlled by the same player. Correctly filters for battlefield creatures that are tapped. Correct.
- Tests: `village_bell_ringer_untaps_creatures` in tier3_cards.rs.

No issues found.
