# Audit: Village Cannibals

## Scryfall Reference
- **Name:** Village Cannibals
- **Cost:** {2}{B}
- **Type:** Creature — Human
- **Oracle:** Whenever another Human creature dies, put a +1/+1 counter on this creature.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/village_cannibals.rs`
- Name: "Village Cannibals" -- MATCH
- Cost: {2}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human"] -- MATCH
- P/T: 2/2 -- MATCH
- Trigger: AnyCreatureDies -- MATCH
- on_any_creature_dies: Checks if dead creature is Human (any controller) -- CORRECT (oracle says "another Human creature", not "another Human creature you control")
- Adds +1/+1 counter -- MATCH

## Verdict
**PASS** — Correctly triggers on any Human creature dying.
