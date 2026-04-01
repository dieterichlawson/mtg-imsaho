## Audit — 2026-04-01

**Scryfall Oracle text**: Bloodcrazed Neonate attacks each combat if able.
Whenever Bloodcrazed Neonate deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire
**Status**: PASS

- Mana cost {1}{R}: correct
- 2/1 stats: correct
- Subtype Vampire: correct
- ForceAttack continuous effect on self: correct
- Triggered ability TriggerKind::CombatDamageToPlayer: correct
- on_combat_damage_to_player adds PlusOnePlusOne counter: correct
- Checks creature is still on battlefield: correct
- Test exists in tier6_cards.rs
