## Audit — 2026-04-01

**Scryfall Oracle text**: Defender\nManor Gargoyle has indestructible as long as it has defender.\n{1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
**Scryfall type line**: Artifact Creature — Gargoyle
**Status**: PASS

- Name: Manor Gargoyle -- correct
- Cost: {5} -- correct
- Type: Artifact Creature -- correct
- Subtypes: Gargoyle -- correct
- P/T: 4/4 -- correct
- Keywords: Defender -- correct
- Conditional indestructible (while has defender) -- correctly implemented via ConditionalKeyword
- Activated ability: {1} to lose defender and gain flying until end of turn -- correctly implemented
- Losing defender also removes indestructible (conditional) -- logic is correct
- Tests exist in tier15_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Defender. Manor Gargoyle has indestructible as long as it has defender. {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
**Scryfall type line**: Artifact Creature -- Gargoyle
**Status**: PASS

No issues found. Scryfall ruling: "Lethal damage dealt to Manor Gargoyle while it has indestructible will stay marked on it that turn. If Manor Gargoyle loses indestructible after having been dealt lethal damage earlier in the turn, it will be destroyed." The implementation handles this correctly because damage_marked persists and SBAs check after defender is lost.
