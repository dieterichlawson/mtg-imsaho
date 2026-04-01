## Audit — 2026-04-01

**Scryfall Oracle text**: When Geistcatcher's Rig enters the battlefield, you may have it deal 4 damage to target creature with flying.
**Scryfall type line**: Artifact Creature — Construct
**Status**: PASS

- Mana cost {6}: correct
- 4/5 stats: correct
- Card types Artifact Creature: correct
- Subtype Construct: correct
- ETB trigger: "you may" correctly modeled as optional=true in ResolutionChoiceKind
- Targets only creatures with flying: correctly filters by has_keyword Flying
- Deals 4 damage: correct
- Tests: no dedicated tests found, but ETB logic is straightforward
