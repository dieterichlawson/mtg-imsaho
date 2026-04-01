## Audit — 2026-04-01

**Scryfall Oracle text**: First strike
Whenever a creature dealt damage by Abattoir Ghoul this turn dies, you gain life equal to that creature's toughness.
**Scryfall type line**: Creature — Zombie
**Status**: PASS

- Mana cost {3}{B}: correct
- 3/2 stats: correct
- Subtype Zombie: correct
- Keyword FirstStrike: correct
- Triggered ability uses TriggerKind::AnyCreatureDies: correct
- on_any_creature_dies checks dead_damaged_by.contains(&self_id): correct
- Life gain uses dead_toughness (last-known information): correct
- Life gain emits LifeChanged event: correct
- Tests exist in tier6_cards.rs covering life gain, no-gain-if-not-damaged, and last-known-toughness scenarios
