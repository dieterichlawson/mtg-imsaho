# Audit: Thraben Purebloods

## Scryfall Reference
- **Name:** Thraben Purebloods
- **Cost:** {4}{W}
- **Type:** Creature — Dog
- **Oracle:** *(no text)*
- **P/T:** 3/5

## Implementation: `mtg-engine/src/cards/thraben_purebloods.rs`
- Name: "Thraben Purebloods" -- MATCH
- Cost: {4}{W} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Dog"] -- MATCH
- P/T: 3/5 -- MATCH
- Oracle: empty -- MATCH
- Keywords: none -- MATCH

## Verdict
**PASS** — Vanilla creature, correctly implemented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: (none — vanilla creature)
**Type line**: Creature — Dog
**Mana Cost**: {4}{W}
**P/T**: 3/5
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Thraben Purebloods", cost {4}{W}, type Creature, subtype Dog, P/T 3/5, no oracle text, no keywords. Vanilla creature correctly implemented.
