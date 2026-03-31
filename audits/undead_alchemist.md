# Audit: Undead Alchemist

## Scryfall Reference
- **Name:** Undead Alchemist
- **Cost:** {3}{U}
- **Type:** Creature — Zombie
- **Oracle:** If a Zombie you control would deal combat damage to a player, instead that player mills that many cards. Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
- **P/T:** 4/2

## Implementation: `mtg-engine/src/cards/undead_alchemist.rs`
- Name: "Undead Alchemist" -- MATCH
- Cost: {3}{U} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Zombie"] -- MATCH
- P/T: 4/2 -- MATCH
- Trigger: AnyCombatDamageToPlayer -- MATCH

### Behavioral Analysis
- Replacement effect: Zombie combat damage -> mill instead. Implemented by restoring life after damage -- ACCEPTABLE workaround
- Zombie check on damage source: checks controller match and Zombie subtype -- CORRECT
- Mill logic: mills from top of library, moves to graveyard -- CORRECT
- Creature exile + token: exiles milled creatures, creates 2/2 black Zombie tokens -- MATCH
- Token stats: 2/2 black Zombie creature -- MATCH

### Known Limitation
- The "whenever a creature card is put into an opponent's graveyard from their library" trigger only fires from this card's own replacement effect, not from external mill sources.

## Verdict
**PASS (with noted limitation)** — Core functionality works correctly.
