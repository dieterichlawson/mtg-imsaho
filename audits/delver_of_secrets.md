# Audit: Delver of Secrets // Insectile Aberration

## Scryfall Reference
- **Front Face: Delver of Secrets**
  - **Cost:** {U}
  - **Type:** Creature -- Human Wizard
  - **Oracle:** At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
  - **P/T:** 1/1

- **Back Face: Insectile Aberration**
  - **Cost:** (none)
  - **Type:** Creature -- Human Insect
  - **Oracle:** Flying
  - **P/T:** 3/2

## Implementation: `delver_of_secrets.rs`
- **Front face name:** Delver of Secrets -- CORRECT
- **Cost:** {U} -- CORRECT
- **Front subtypes:** ["Human", "Wizard"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Insectile Aberration -- CORRECT
- **Back subtypes:** ["Human", "Insect"] -- CORRECT
- **Back P/T:** 3/2 -- CORRECT
- **Back keywords:** [Flying] -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Checks top card of library for instant/sorcery, transforms if found -- CORRECT

## Issues
None
