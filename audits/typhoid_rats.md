# Audit: Typhoid Rats

## Scryfall Reference
- **Name:** Typhoid Rats
- **Cost:** {B}
- **Type:** Creature — Rat
- **Oracle:** Deathtouch
- **P/T:** 1/1

## Implementation: `mtg-engine/src/cards/typhoid_rats.rs`
- Name: "Typhoid Rats" -- MATCH
- Cost: {B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Rat"] -- MATCH
- P/T: 1/1 -- MATCH
- Keywords: [Deathtouch] -- MATCH

## Verdict
**PASS** — Simple deathtouch creature, correctly implemented.
