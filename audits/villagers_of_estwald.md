# Audit: Villagers of Estwald // Howlpack of Estwald

## Scryfall Reference
### Front Face
- **Name:** Villagers of Estwald
- **Cost:** {2}{G}
- **Type:** Creature — Human Werewolf
- **Oracle:** At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **P/T:** 2/3

### Back Face
- **Name:** Howlpack of Estwald
- **Cost:** *(none)*
- **Type:** Creature — Werewolf
- **Oracle:** At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
- **P/T:** 4/6

## Implementation: `mtg-engine/src/cards/villagers_of_estwald.rs`

### Front Face
- Name: "Villagers of Estwald" -- MATCH
- Cost: {2}{G} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Werewolf"] -- MATCH
- P/T: 2/3 -- MATCH
- Trigger: Upkeep -- MATCH

### Back Face
- Name: "Howlpack of Estwald" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Werewolf"] -- MATCH
- P/T: 4/6 -- MATCH

## Verdict
**PASS** — Standard Innistrad werewolf, correctly implemented.
