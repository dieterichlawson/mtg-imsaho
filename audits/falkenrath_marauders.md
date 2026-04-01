## Audit — 2026-04-01

**Scryfall Oracle text**: Flying, haste\nWhenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on Falkenrath Marauders.
**Scryfall type line**: Creature — Vampire Warrior
**Status**: PASS

- Mana cost {3}{R}{R}: correct.
- Type Creature, subtypes Vampire Warrior: correct.
- Power/Toughness 2/2: correct.
- Keywords: Flying, Haste: correct.
- Trigger on combat damage to player (TriggerKind::CombatDamageToPlayer): correct.
- Adds 2 +1/+1 counters: correct.
- Checks zone == Battlefield before adding counters: correct.
- Tests exist in `tier6_cards.rs` (`falkenrath_marauders_two_counters_on_combat_damage`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying, haste. Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
**Scryfall type line**: Creature — Vampire Warrior
**Status**: PASS

No issues found.
