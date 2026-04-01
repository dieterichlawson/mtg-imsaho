## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, {T}: Tap target non-Human creature.
**Scryfall type line**: Creature — Human Cleric
**Status**: PASS

- Mana cost {1}{W}: correct
- 1/2 stats: correct
- Subtypes Human, Cleric: correct
- Activated ability: {1}, tap, target non-Human creature: correct
- requires_tap: true: correct
- is_valid_target excludes Humans and requires battlefield/creature: correct
- on_activate_ability sets target tapped = true: correct
- sorcery_speed_only: false — correct, this can be activated at instant speed
- Tests exist in activated_abilities.rs covering stats, tapping, human exclusion, and tap requirement

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: {1}, {T}: Tap target non-Human creature.
**Scryfall type line**: Creature — Human Cleric
**Status**: PASS

No issues found. Activated ability cost, tap requirement, Human exclusion, and target filtering all correct.
