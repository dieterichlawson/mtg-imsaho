## Audit — 2026-04-01

**Scryfall Oracle text**: Lava Axe deals 5 damage to target player or planeswalker.
**Scryfall type line**: Sorcery
**Status**: ISSUE

**Findings**:

1. Name: Lava Axe -- correct
2. Cost: {4}{R} -- correct
3. Type: Sorcery -- correct
4. Damage: 5 -- correct
5. **ISSUE — Target restriction too narrow**: The Oracle text says "target player or planeswalker" but the implementation uses `TargetRequirement::PlayerOnly`, which does not allow targeting planeswalkers. Should be `PlayerOrPlaneswalker` or equivalent.
6. The oracle_text field in the implementation correctly says "target player or planeswalker" but the target_requirement() method only returns PlayerOnly.
7. Tests exist in spells.rs (only tests player targeting).

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Lava Axe deals 5 damage to target player or planeswalker.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Confirmed: target_requirement() returns PlayerOnly but Oracle says "target player or planeswalker." Should allow planeswalker targeting. This would matter when Liliana of the Veil or other planeswalkers are on the battlefield.
