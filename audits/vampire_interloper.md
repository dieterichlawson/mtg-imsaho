## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nVampire Interloper can't block.
**Scryfall type line**: Creature — Vampire Scout
**Scryfall mana cost**: {1}{B}
**Scryfall P/T**: 2/1
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {1}{B} — correct.
- Types: Creature — Vampire Scout — correct.
- P/T: 2/1 — correct.
- Keywords: Flying — correct.
- Can't block: Implemented via `ContinuousEffect::PreventBlock { scope: EffectScope::OnSelf }`. Correct.
- Tests: `vampire_interloper_cant_block` in card_mechanics.rs.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. Vampire Interloper can't block.
**Scryfall type line**: Creature — Vampire Scout
**P/T**: 2/1, **Mana cost**: {1}{B}
**Status**: PASS

No issues found. Subtypes (Vampire, Scout), flying keyword, and PreventBlock continuous effect all correct.
