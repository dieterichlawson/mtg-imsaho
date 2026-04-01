## Audit — 2026-04-01

**Scryfall Oracle text**: Haste\n{1}{B}: Regenerate Manor Skeleton. (The next time this creature would be destroyed this turn, it isn't. Instead tap it, remove all damage from it, and remove it from combat.)
**Scryfall type line**: Creature — Skeleton
**Status**: PASS

- Name: Manor Skeleton -- correct
- Cost: {1}{B} -- correct
- Type: Creature -- correct
- Subtypes: Skeleton -- correct
- P/T: 1/1 -- correct
- Keywords: Haste -- correct
- Activated ability: {1}{B} to regenerate -- correctly implemented (adds regeneration shield)
- Ability does not require tap -- correct
- Tests exist in activated_abilities.rs (stats, ability, and lethal damage tests)

No issues found. Implementation matches Oracle text.
