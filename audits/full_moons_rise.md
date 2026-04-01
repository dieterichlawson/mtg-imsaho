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
