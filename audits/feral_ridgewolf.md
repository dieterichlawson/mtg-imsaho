## Audit — 2026-04-01

**Scryfall Oracle text**: Trample\n{1}{R}: Feral Ridgewolf gets +2/+0 until end of turn.
**Scryfall type line**: Creature — Wolf
**Status**: PASS

- Mana cost {2}{R}: correct.
- Type Creature, subtype Wolf: correct.
- Power/Toughness 1/2: correct.
- Keywords: Trample: correct.
- Activated ability cost {1}{R}: correct.
- Grants +2/+0 until end of turn via UntilEndOfTurnEffect: correct.
- `requires_tap: false`: correct.
- Can activate multiple times: correct (`once_per_turn: false`).
- Only available on battlefield: correct.
- Tests exist in `activated_abilities.rs` (`feral_ridgewolf_has_correct_stats`, `feral_ridgewolf_gets_plus_2_plus_0`, `feral_ridgewolf_can_activate_multiple_times`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Trample. {1}{R}: This creature gets +2/+0 until end of turn.
**Scryfall type line**: Creature — Wolf
**Status**: PASS

No issues found.
