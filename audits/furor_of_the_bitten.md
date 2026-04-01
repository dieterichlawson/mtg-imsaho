## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

- Mana cost {R}: correct.
- Type Enchantment, subtype Aura: correct.
- Targets creature for attachment: correct.
- +2/+2 buff via `ModifyPT { power: 2, toughness: 2, scope: Attached }`: correct.
- Force attack via `ForceAttack { scope: Attached }`: correct.
- Uses `resolve_aura` helper: correct.
- Tests exist in `bug_fixes.rs` (`furor_of_the_bitten_gives_plus_two_and_forces_attack`) and `innistrad_cards.rs`.
