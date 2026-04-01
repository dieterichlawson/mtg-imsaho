## Audit — 2026-04-01

**Scryfall Oracle text**: Werewolf creatures you control get +1/+0 and have trample.\nSacrifice Full Moon's Rise: Regenerate all Werewolf and Wolf creatures you control.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Mana cost {1}{G}: correct.
- Type Enchantment: correct.
- Sacrifice ability with `SacrificeCost::SacrificeThis`: correct.
- Regeneration effect grants `regeneration_shields += 1`: correct.

**Issue 1 — Static buff only applies to Werewolf, not Wolf creatures.** The Oracle text says "Werewolf creatures you control get +1/+0 and have trample" but the actual Innistrad printing says "Werewolf creatures you control get +1/+0 and have trample." The implementation's continuous_effects only filter for `HasSubtype("Werewolf")` which matches the Oracle text. However, the oracle_text field in the code also only says "Werewolf creatures" for the static buff, which is correct.

**Issue 2 — Regeneration ability only regenerates Werewolf creatures, not Wolf creatures.** The Oracle for the sacrifice ability says "Regenerate all Werewolf and Wolf creatures you control." The `on_activate_ability` implementation only checks for `s == "Werewolf"` (line 84) and does NOT check for "Wolf". Wolf creatures would not be regenerated. The description string mentions "Wolf and Werewolf" but the filter code only matches Werewolf.

- Tests exist in `innistrad_simple_cards.rs` (`full_moons_rise_card_data`) — card data only, no behavioral tests.

## Audit — 2026-04-01

**Scryfall Oracle text**: Werewolf creatures you control get +1/+0 and have trample. Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
**Scryfall type line**: Enchantment
**Status**: ISSUE

1. **Activated ability description mentions "Wolf" incorrectly**: The activated ability description on line 58 says "Sacrifice: Regenerate all Wolf and Werewolf creatures you control" but Oracle text only mentions "Werewolf creatures". Wolf is NOT included. The actual filter code on line 84 correctly only checks for "Werewolf", so behavior is correct but description is misleading. File: `mtg-engine/src/cards/full_moons_rise.rs`, line 58.
2. **Code comment mentions "Wolf" incorrectly**: The doc comment on line 9 says "Werewolf and Wolf creatures" but Oracle only says "Werewolf creatures". File: `mtg-engine/src/cards/full_moons_rise.rs`, line 9.
