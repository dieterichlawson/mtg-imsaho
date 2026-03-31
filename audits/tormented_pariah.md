# Audit: Tormented Pariah // Rampaging Werewolf

## Scryfall Reference
### Front Face
- **Name:** Tormented Pariah
- **Cost:** {3}{R}
- **Type:** Creature — Human Warrior Werewolf
- **Oracle:** At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **P/T:** 3/2

### Back Face
- **Name:** Rampaging Werewolf
- **Cost:** *(none)*
- **Type:** Creature — Werewolf
- **Oracle:** At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
- **P/T:** 6/4

## Implementation: `mtg-engine/src/cards/tormented_pariah.rs`

### Front Face
- Name: "Tormented Pariah" -- MATCH
- Cost: {3}{R} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Warrior", "Werewolf"] -- MATCH
- P/T: 3/2 -- MATCH
- Trigger: Upkeep -- MATCH

### Back Face
- Name: "Rampaging Werewolf" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Werewolf"] -- MATCH
- P/T: 6/4 -- MATCH

### Behavioral Notes
- Standard werewolf transform logic: no spells -> transform to back; any player 2+ spells -> transform to front.
- Uses is_first_turn guard to prevent transform on turn 1 (correct).

## Verdict
**PASS** — Standard Innistrad werewolf, correctly implemented.
