## Audit — 2026-04-01

**Scryfall Oracle text (front)**: First strike\nAt the beginning of each upkeep, if no spells were cast last turn, transform Village Ironsmith.
**Scryfall Oracle text (back — Ironfang)**: First strike\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Ironfang.
**Scryfall type line**: Creature — Human Werewolf // Creature — Werewolf
**Scryfall mana cost**: {1}{R}
**Scryfall P/T**: 1/1 // 3/1
**Status**: PASS

Findings:
- Name: Correct ("Village Ironsmith" / "Ironfang").
- Mana cost: {1}{R} — correct.
- Types: Creature — Human Werewolf // Creature — Werewolf — correct.
- P/T: 1/1 // 3/1 — correct. `dynamic_pt` returns (3,1) when transformed.
- Keywords: First strike on both faces — correct.
- Transform logic: Same werewolf pattern as other cards; correct.
- Tests: `village_ironsmith_keeps_first_strike_on_both_faces` in werewolf_cards.rs.

No issues found.
