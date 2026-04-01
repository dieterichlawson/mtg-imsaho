## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Tormented Pariah.
**Scryfall Oracle text (back, Rampaging Werewolf)**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform Rampaging Werewolf.
**Scryfall type line (front)**: Creature — Human Warrior Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: PASS

- Name: correct ("Tormented Pariah" / "Rampaging Werewolf")
- Cost: {3}{R} -- correct
- Type: Creature -- correct
- Subtypes front: Human, Warrior, Werewolf -- correct
- Subtypes back: Werewolf -- correct
- Front P/T: 3/2 -- correct
- Back P/T: 6/4 -- correct
- Standard werewolf transform logic implemented correctly:
  - Front->back: no spells cast last turn, not first turn
  - Back->front: any player cast 2+ spells last turn
- `dynamic_pt` returns (6, 4) when transformed -- correct
- `on_upkeep` handles transform in both directions -- correct
- Tests exist in `werewolf_cards.rs`
- No issues found
