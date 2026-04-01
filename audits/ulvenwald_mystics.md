## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Ulvenwald Mystics.
**Scryfall Oracle text (back — Ulvenwald Primordials)**: {G}: Regenerate Ulvenwald Primordials. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Ulvenwald Primordials.
**Scryfall type line**: Creature — Human Shaman Werewolf // Creature — Werewolf
**Scryfall mana cost**: {2}{G}{G}
**Scryfall P/T**: 3/3 // 5/5
**Status**: PASS

Findings:
- Name: Correct ("Ulvenwald Mystics").
- Mana cost: {2}{G}{G} — correct (Generic(2), Green, Green).
- Types: Creature — Human Shaman Werewolf — correct.
- P/T: 3/3 front, 5/5 back — correct. `dynamic_pt` returns (5,5) when transformed.
- Oracle text: Matches for both faces.
- Transform logic: Correctly checks `spells_cast_last_turn == 0` for front face, and `any player cast >= 2` for back face. First turn excluded.
- Back face: Ulvenwald Primordials with {G}: Regenerate — correctly implemented via activated ability that adds regeneration shields.
- Back face subtypes: "Werewolf" only — correct.
- Tests: `ulvenwald_mystics_transforms_and_gains_regenerate` in werewolf_cards.rs covers core behavior.

No issues found.
