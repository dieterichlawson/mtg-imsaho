## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Grizzled Outcasts) At the beginning of each upkeep, if no spells were cast last turn, transform Grizzled Outcasts.
(Back — Krallenhorde Wantons) At the beginning of each upkeep, if a player cast two or more spells last turn, transform Krallenhorde Wantons.
**Scryfall type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: PASS

- Mana cost {4}{G}: correct
- Front face 4/4: correct
- Front face subtypes Human Werewolf: correct
- Back face name "Krallenhorde Wantons": correct
- Back face 7/7: correct
- Back face subtypes Werewolf: correct
- Werewolf transform logic: correct
- dynamic_pt returns (7,7) when transformed: correct
- Tests exist in werewolf_cards.rs
