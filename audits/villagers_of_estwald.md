## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Villagers of Estwald.
**Scryfall Oracle text (back — Howlpack of Estwald)**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack of Estwald.
**Scryfall type line**: Creature — Human Werewolf // Creature — Werewolf
**Scryfall mana cost**: {2}{G}
**Scryfall P/T**: 2/3 // 4/6
**Status**: PASS

Findings:
- Name: Correct ("Villagers of Estwald" / "Howlpack of Estwald").
- Mana cost: {2}{G} — correct.
- Types: Creature — Human Werewolf // Creature — Werewolf — correct.
- P/T: 2/3 // 4/6 — correct. `dynamic_pt` returns (4,6) when transformed.
- Transform logic: Standard werewolf pattern, correct.
- Tests: `villagers_of_estwald_transforms_to_large_body` in werewolf_cards.rs.

No issues found.
