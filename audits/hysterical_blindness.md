## Audit — 2026-04-01

**Scryfall Oracle text**: Creatures your opponents control get -4/-0 until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {2}{U}: correct
- Card type Instant: correct
- On resolve: applies -4/+0 until end of turn to all creatures opponents control: correct
- Uses UntilEndOfTurnEffect with power_mod=-4, toughness_mod=0: correct
- Tests exist in innistrad_cards.rs covering opponent creature debuff

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Creatures your opponents control get -4/-0 until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

No issues found. Correctly uses until_end_of_turn_effects with -4 power, 0 toughness. Only affects creatures on battlefield at resolution time (per Scryfall ruling). Uses move_spell_after_resolve.
