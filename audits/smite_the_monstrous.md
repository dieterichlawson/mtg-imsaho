## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target creature with power 4 or greater.
**Scryfall type line**: Instant
**Mana cost**: {3}{W}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {3}{W}, type Instant
- Target requirement: creature with power >= 4 (uses `TargetFilter::PowerAtLeast(4)`)
- `is_valid_target` checks effective power (accounts for buffs/debuffs/counters)
- Resolution delegates to `helpers::resolve_destroy`
- Tests: `smite_the_monstrous_kills_big_creature` in tier2_spells.rs

No issues found.
