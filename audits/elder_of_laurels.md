## Audit — 2026-04-01

**Scryfall Oracle text**: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
**Scryfall type line**: Creature — Human Advisor
**Status**: PASS

- Mana cost {2}{G}: correct.
- Type Creature, subtypes Human Advisor: correct.
- Power/Toughness 2/3: correct.
- Activated ability cost {3}{G}: correct.
- Targets any creature (TargetRequirement::Creature): correct.
- X = number of creatures you control: correct (counts by `power.is_some()`).
- Applies as UntilEndOfTurnEffect with power_mod and toughness_mod: correct.
- `requires_tap: false`: correct (no tap in cost).
- Only available on battlefield: correct.
- Tests exist in `tier10_cards.rs` (`elder_of_laurels_card_data`, `elder_of_laurels_pumps_by_creature_count`).
