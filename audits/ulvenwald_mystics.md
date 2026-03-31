# Audit: Ulvenwald Mystics // Ulvenwald Primordials

## Scryfall Reference
### Front Face
- **Name:** Ulvenwald Mystics
- **Cost:** {2}{G}{G}
- **Type:** Creature — Human Shaman Werewolf
- **Oracle:** At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **P/T:** 3/3

### Back Face
- **Name:** Ulvenwald Primordials
- **Cost:** *(none)*
- **Type:** Creature — Werewolf
- **Oracle:** {G}: Regenerate this creature. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
- **P/T:** 5/5

## Implementation: `mtg-engine/src/cards/ulvenwald_mystics.rs`

### Front Face
- Name: "Ulvenwald Mystics" -- MATCH
- Cost: {2}{G}{G} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Shaman", "Werewolf"] -- MATCH
- P/T: 3/3 -- MATCH
- Trigger: Upkeep -- MATCH

### Back Face
- Name: "Ulvenwald Primordials" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Werewolf"] -- MATCH
- P/T: 5/5 -- MATCH
- Activated ability: {G}: Regenerate (only when transformed) -- MATCH
- Regeneration shield implementation -- CORRECT

## Verdict
**PASS** — Werewolf with regenerate ability correctly implemented.
