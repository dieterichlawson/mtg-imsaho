## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{R}: correct
- Card type Instant: correct
- Flashback {G}: correct
- Target requirement: PermanentWithFilter(HasCardType(Artifact)): correct
- is_valid_target checks for Artifact card type on battlefield: correct
- on_resolve uses resolve_destroy helper (which uses try_destroy pipeline): correct
- Uses move_spell_after_resolve (via helper): correct

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Destroy target artifact. Flashback {G}
**Scryfall type line**: Instant
**Status**: ISSUE

1. **No tests**: No test files reference Ancient Grudge. Missing tests for basic artifact destruction, flashback from graveyard for {G}, exile after flashback, fizzle when target removed.
2. **Not in LLM card knowledge**: Missing from mtg-player/src/llm.rs card knowledge section.
