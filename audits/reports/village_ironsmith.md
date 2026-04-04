# Audit: Village Ironsmith // Ironfang

## Scryfall Reference
### Front Face
- **Name:** Village Ironsmith
- **Cost:** {1}{R}
- **Type:** Creature — Human Werewolf
- **Oracle:** First strike / At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **P/T:** 1/1

### Back Face
- **Name:** Ironfang
- **Cost:** *(none)*
- **Type:** Creature — Werewolf
- **Oracle:** First strike / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
- **P/T:** 3/1

## Implementation: `mtg-engine/src/cards/village_ironsmith.rs`

### Front Face
- Name: "Village Ironsmith" -- MATCH
- Cost: {1}{R} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Werewolf"] -- MATCH
- P/T: 1/1 -- MATCH
- Keywords: [FirstStrike] -- MATCH
- Trigger: Upkeep -- MATCH

### Back Face
- Name: "Ironfang" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Werewolf"] -- MATCH
- P/T: 3/1 -- MATCH
- Keywords: [FirstStrike] -- MATCH (both faces have first strike)

## Verdict
**PASS** — Standard Innistrad werewolf with first strike, correctly implemented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Front oracle text**: First strike / At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back oracle text**: First strike / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Mana cost**: {1}{R}
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**P/T**: 1/1 // 3/1
**Status**: PASS
### Code issues
None. Card data matches oracle for both faces. Front: name "Village Ironsmith", cost {1}{R}, 1/1, Human Werewolf, First strike, transform trigger on upkeep. Back: name "Ironfang", 3/1, Werewolf, First strike, transform back trigger. Transform logic correctly checks spells_cast_last_turn == 0 for front face and any player with >= 2 spells for back face. dynamic_pt returns (3,1) when transformed. All correct.
