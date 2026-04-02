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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Front oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back oracle text**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Mana cost**: {2}{G}
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**P/T**: 2/3 // 4/6
**Status**: PASS
### Code issues
None. Card data matches oracle for both faces. Front: name "Villagers of Estwald", cost {2}{G}, 2/3, Human Werewolf, transform trigger on upkeep. Back: name "Howlpack of Estwald", 4/6, Werewolf, transform back trigger. Transform logic correctly checks spells_cast_last_turn == 0 for front face and any player with >= 2 spells for back face. dynamic_pt returns (4,6) when transformed. No keywords (vanilla werewolf). All correct.
