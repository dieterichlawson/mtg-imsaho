## Audit — 2026-04-01

**Scryfall Oracle text**: Geistflame deals 1 damage to any target.
Flashback {3}{R}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {R}: correct
- Card type Instant: correct
- Target requirement AnyTarget: correct
- Deals 1 damage via resolve_damage helper: correct
- Flashback cost {3}{R}: correct
- Tests exist in tier2_spells.rs, flashback.rs, and fizzle.rs covering damage, flashback casting, and flashback exile
