## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Gatstaf Shepherd) At the beginning of each upkeep, if no spells were cast last turn, transform Gatstaf Shepherd.
(Back — Gatstaf Howler) Intimidate
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Gatstaf Howler.
**Scryfall type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: PASS

- Mana cost {1}{G}: correct
- Front face 2/2: correct
- Front face subtypes Human Werewolf: correct
- Back face name "Gatstaf Howler": correct
- Back face 3/3: correct
- Back face subtypes Werewolf: correct
- Back face keyword Intimidate: correct
- Werewolf transform logic (no spells last turn -> transform to back; 2+ spells last turn -> transform to front): correct
- on_upkeep correctly toggles is_transformed and updates name
- dynamic_pt returns (3,3) when transformed: correct
- Tests exist in werewolf_cards.rs covering transform and intimidate gain/loss
