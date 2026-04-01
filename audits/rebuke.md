## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target attacking creature.
**Scryfall type line**: Instant
**Mana cost**: {2}{W}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}{W}, type Instant
- Target requirement: attacking creature (uses `TargetFilter::Attacking`)
- `is_valid_target` checks the creature is on the battlefield, is a creature (has power), and is in the attackers list
- Resolution delegates to `helpers::resolve_destroy`
- Tests: `rebuke_destroys_attacking_creature` in tier2_spells.rs

No issues found.
