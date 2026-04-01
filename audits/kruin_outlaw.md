## Audit — 2026-04-01

**Scryfall Oracle text (front — Kruin Outlaw)**: First strike\nAt the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Scryfall Oracle text (back — Terror of Kruin Pass)**: Double strike, menace\nEach Werewolf you control can't be blocked except by two or more creatures.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Scryfall type line**: Creature — Human Rogue Werewolf // Creature — Werewolf
**Status**: ISSUE

**Findings**:

1. **Front face**: Name, cost ({1}{R}{R}), types (Human Rogue Werewolf), P/T (2/2), first strike, and transform trigger all correct.
2. **Back face**: Name (Terror of Kruin Pass), P/T (3/3), double strike, menace -- correct.
3. **ISSUE — Missing "can't be blocked except by two" ability on back face**: The Oracle text for Terror of Kruin Pass includes "Each Werewolf you control can't be blocked except by two or more creatures." This is a global evasion ability for all your Werewolves. The implementation only grants double strike and menace to Terror of Kruin Pass itself but does not implement the "can't be blocked except by two or more creatures" ability for other Werewolves. Note that menace on Terror of Kruin Pass itself covers the blocking restriction for this creature, but other Werewolves you control should also gain this restriction.
4. Transform logic is correct (standard werewolf transform conditions).
5. Tests exist in werewolf_cards.rs.

**Summary**: The back face is missing a continuous effect that grants all Werewolves you control the "can't be blocked except by two or more creatures" evasion ability. This is functionally distinct from menace (menace is only on Terror of Kruin Pass itself).

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text (front)**: First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Scryfall Oracle text (back)**: Double strike, menace. Each Werewolf you control can't be blocked except by two or more creatures. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Scryfall type line**: Creature -- Human Rogue Werewolf // Creature -- Werewolf
**Status**: ISSUE

- Confirmed: Back face is missing a global continuous effect granting all Werewolves "can't be blocked except by two or more creatures." Menace on Terror of Kruin Pass itself is correct, but the evasion for OTHER Werewolves is not implemented.
- Note: The back face Oracle text says "double strike, menace" -- the implementation correctly has both keywords.
- P/T front 2/2, back 3/3 -- correct.
