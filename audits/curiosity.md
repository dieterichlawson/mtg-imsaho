## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Scryfall type line**: Enchantment — Aura
**Status**: ISSUE

### Findings

1. **Trigger kind may be too narrow (potential ISSUE)**: Uses `TriggerKind::AnyDamageToPlayer` which is good. The `on_any_damage_to_player` handler correctly checks that the source is the enchanted creature and that the damaged player is an opponent. However, Oracle says "deals damage" (any damage, including combat and non-combat). The handler receives damage events — need to verify that both combat and non-combat damage events invoke this hook. If `AnyDamageToPlayer` only covers one type, this would be a bug.

2. **"You may" is auto-resolved (minor ISSUE)**: Oracle says "you **may** draw a card." The implementation auto-draws (line 65). While drawing is almost always beneficial, there are edge cases (e.g., near-empty library to avoid decking).

3. **Card data correct**: Name, cost ({U}), type (Enchantment), subtype (Aura) all match.

4. **Opponent check correct**: Line 61 correctly checks `damaged_player == controller` and returns if true (only triggers on opponents).

5. **Tests**: Found in `tier6_cards.rs`.
