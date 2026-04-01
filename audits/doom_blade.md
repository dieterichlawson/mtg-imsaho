## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target nonblack creature.
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{B}: correct.
- Type Instant: correct.
- Target: nonblack creature via `TargetFilter::Nonblack`: correct.
- `is_valid_target` checks battlefield, power.is_some() (creature check), and `!o.colors.contains(&Color::Black)`: correct.
- Uses `resolve_destroy` helper which calls `try_destroy`: correct pipeline for "destroy".
- `resolve_destroy` calls `move_spell_after_resolve`: correct.
- Tests exist in `spells.rs` (`doom_blade_destroys`) and `spell_fizzle.rs` (`doom_blade_target_already_gone`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target nonblack creature.
**Scryfall type line**: Instant
**Status**: PASS

No issues found.
