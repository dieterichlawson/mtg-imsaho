## Audit — 2026-04-01

**Scryfall Oracle text**: Lightning Bolt deals 3 damage to any target.
**Scryfall type line**: Instant
**Status**: PASS

- Name: Lightning Bolt -- correct
- Cost: {R} -- correct
- Type: Instant -- correct
- Damage: 3 -- correct
- Target: AnyTarget (creature, player, or planeswalker) -- correct
- Tests exist in spells.rs (tests creature kill and player damage)

No issues found. Implementation matches Oracle text.
