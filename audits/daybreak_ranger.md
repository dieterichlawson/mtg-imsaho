# Audit: Daybreak Ranger // Nightfall Predator

## Scryfall Reference
- **Front Face: Daybreak Ranger**
  - **Cost:** {2}{G}
  - **Type:** Creature -- Human Archer Ranger Werewolf
  - **Oracle:** {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
  - **P/T:** 2/2

- **Back Face: Nightfall Predator**
  - **Cost:** (none)
  - **Type:** Creature -- Werewolf
  - **Oracle:** {R}, {T}: This creature fights target creature. At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
  - **P/T:** 4/4

## Implementation: `daybreak_ranger.rs`
- **Front face name:** Daybreak Ranger -- CORRECT
- **Cost:** {2}{G} -- CORRECT
- **Front subtypes:** ["Human", "Archer", "Werewolf"] -- ISSUE (see below)
- **Front P/T:** 2/2 -- CORRECT
- **Back face name:** Nightfall Predator -- CORRECT
- **Back subtypes:** ["Werewolf"] -- CORRECT
- **Back P/T:** 4/4 -- CORRECT
- **Front ability:** {T}: 2 damage to flying creature -- CORRECT
- **Back ability:** {R}, {T}: fight target creature -- CORRECT
- **Transform logic:** Werewolf standard (no spells / 2+ spells) -- CORRECT
- **Uses NonCombatDamageDealt for front ability:** Yes -- CORRECT

## Issues
1. **ISSUE: Missing "Ranger" subtype on front face.** Scryfall type line is "Creature -- Human Archer Ranger Werewolf" but implementation has subtypes ["Human", "Archer", "Werewolf"] -- missing "Ranger".
