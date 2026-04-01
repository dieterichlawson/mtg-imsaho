## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature.
**Scryfall type line**: Creature — Elemental Dog
**Status**: PASS

- Mana cost {1}{R}: correct
- 2/1 stats: correct
- Subtypes: ["Elemental", "Dog"] — correct. The card was originally printed as "Elemental Hound" but received errata to "Elemental Dog" per the 2021 creature type update.
- Triggered abilities: TriggerKind::Blocks and TriggerKind::BecomesBlocked — correct
- on_blocks deals 1 damage to blocked_attacker: correct
- on_becomes_blocked deals 1 damage to blocker_id: correct
- Damage uses NonCombatDamageDealt event: correct (this is triggered ability damage, not combat damage)
- Test exists in tier12_cards.rs
