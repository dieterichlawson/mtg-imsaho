## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
Whenever Balefire Dragon deals combat damage to a player, it deals that much damage to each creature that player controls.
**Scryfall type line**: Creature — Dragon
**Status**: PASS

- Mana cost {5}{R}{R}: correct
- 6/6 stats: correct
- Subtype Dragon: correct
- Keyword Flying: correct
- Triggered ability TriggerKind::CombatDamageToPlayer: correct
- on_combat_damage_to_player collects all creatures the damaged player controls: correct
- Deals `amount` damage (the combat damage dealt) to each creature: correct
- Uses NonCombatDamageDealt event for the triggered ability damage: correct (this follow-up damage is not combat damage)
- Checks self is still on battlefield before triggering: correct
- Test exists in tier6_cards.rs

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Flying. Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
**Scryfall type line**: Creature — Dragon
**Status**: PASS

No issues found. Triggered damage correctly uses NonCombatDamageDealt (per Scryfall ruling: "The damage dealt by Balefire Dragon's triggered ability isn't combat damage").
