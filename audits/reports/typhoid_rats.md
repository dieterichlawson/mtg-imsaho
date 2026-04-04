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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Deathtouch
**Type line**: Creature — Rat
**Status**: PASS

### Card Data
- **Name:** Typhoid Rats -- CORRECT
- **Mana Cost:** {B} -- CORRECT
- **Type:** Creature — Rat -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** Deathtouch -- CORRECT

### Code issues
None. Vanilla creature with Deathtouch keyword. All card data matches oracle.
