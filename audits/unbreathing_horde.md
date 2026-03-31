# Audit: Unbreathing Horde

## Scryfall Reference
- **Name:** Unbreathing Horde
- **Cost:** {2}{B}
- **Type:** Creature — Zombie
- **Oracle:** This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard. If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
- **P/T:** 0/0

## Implementation: `mtg-engine/src/cards/unbreathing_horde.rs`
- Name: "Unbreathing Horde" -- MATCH
- Cost: {2}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Zombie"] -- MATCH
- P/T: 0/0 -- MATCH
- Trigger: EntersBattlefield -- MATCH

### ETB Counter Logic
- Counts other Zombies on battlefield under controller (excludes self) -- MATCH
- Counts Zombie cards in controller's graveyard -- MATCH
- Adds +1/+1 counters equal to total -- MATCH

### ISSUE: Missing Damage Prevention
- Oracle: "If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it."
- The implementation does NOT implement this damage replacement effect. The code comment mentions indestructible as an approximation, but neither indestructible nor the damage prevention is actually implemented.
- **BUG**: The creature takes damage normally instead of preventing it and removing a counter. This changes the card's behavior significantly -- it should survive any single damage event (removing 1 counter regardless of damage amount), but instead it takes lethal damage as normal.

## Verdict
**FAIL** — Missing damage prevention replacement effect. ETB counters work correctly.
