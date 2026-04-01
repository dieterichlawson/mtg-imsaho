## Audit — 2026-04-01

**Scryfall Oracle text (front — Mayor of Avabruck)**: Other Human creatures you control get +1/+1.\nAt the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Scryfall Oracle text (back — Howlpack Alpha)**: Each other Werewolf and Wolf creature you control gets +1/+1.\nAt the beginning of your end step, create a 2/2 green Wolf creature token.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Scryfall type line**: Creature — Human Advisor Werewolf // Creature — Werewolf
**Status**: PASS

- Name (front): Mayor of Avabruck -- correct
- Cost: {1}{G} -- correct
- Subtypes (front): Human, Advisor, Werewolf -- correct
- P/T (front): 1/1 -- correct
- Front face buff: other Human creatures +1/+1 -- correctly implemented
- Name (back): Howlpack Alpha -- correct
- Subtypes (back): Werewolf -- correct
- P/T (back): 3/3 -- correct
- Back face buff: other Werewolf and Wolf creatures +1/+1 -- correctly implemented
- End step token creation (back face only, controller's end step): 2/2 green Wolf -- correct
- Transform logic: standard werewolf transform -- correct
- Tests exist in werewolf_cards.rs

No issues found. Implementation matches Oracle text.
