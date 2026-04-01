## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Reckless Waif.
**Scryfall Oracle text (back — Merciless Predator)**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform Merciless Predator.
**Scryfall type line**: Creature — Human Rogue Werewolf // Creature — Werewolf
**Mana cost**: {R}
**P/T**: 1/1 // 3/2
**Status**: PASS

Implementation correctly models:
- Front face: name, cost {R}, subtypes Human/Rogue/Werewolf, P/T 1/1
- Back face: Merciless Predator, subtypes Werewolf, P/T 3/2
- Transform condition (front): no spells cast last turn AND not first turn
- Transform condition (back): any player cast 2+ spells last turn
- `on_upkeep` triggers transformation and updates name
- `dynamic_pt` returns (3,2) when transformed
- Tests: 4 tests in werewolf_cards.rs covering transform/untransform logic

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Reckless Waif.
**Scryfall Oracle text (back)**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform Merciless Predator.
**Scryfall type line (front)**: Creature — Human Rogue Werewolf (1/1)
**Scryfall type line (back)**: Creature — Werewolf (3/2)
**Status**: PASS

No issues found. Transform logic, subtypes, P/T, and DFC data all correct.
