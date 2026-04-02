# Audit: Walking Corpse

## Scryfall Reference
- **Name:** Walking Corpse
- **Cost:** {1}{B}
- **Type:** Creature — Zombie
- **Oracle:** *(no text)*
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/walking_corpse.rs`
- Name: "Walking Corpse" -- MATCH
- Cost: {1}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Zombie"] -- MATCH
- P/T: 2/2 -- MATCH
- Oracle: empty -- MATCH

## Verdict
**PASS** — Vanilla creature, correctly implemented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: (none — vanilla creature)
**Mana cost**: {1}{B}
**Type line**: Creature — Zombie
**P/T**: 2/2
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Walking Corpse", cost {1}{B}, 2/2, type Creature — Zombie, no keywords, empty oracle text. Vanilla creature, no behavior needed beyond card_data. All correct.
