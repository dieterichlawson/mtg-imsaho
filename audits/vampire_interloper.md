# Audit: Vampire Interloper

## Scryfall Reference
- **Name:** Vampire Interloper
- **Cost:** {1}{B}
- **Type:** Creature — Vampire Scout
- **Oracle:** Flying / This creature can't block.
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/vampire_interloper.rs`
- Name: "Vampire Interloper" -- MATCH
- Cost: {1}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Vampire", "Scout"] -- MATCH
- P/T: 2/1 -- MATCH
- Keywords: [Flying] -- MATCH
- Continuous effects: [PreventBlock { scope: OnSelf }] -- MATCH ("can't block")

## Verdict
**PASS** — Correctly implemented with flying and can't-block restriction.
