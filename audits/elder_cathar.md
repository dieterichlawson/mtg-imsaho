## Audit — 2026-04-01

**Scryfall Oracle text**: When Elder Cathar dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Scryfall type line**: Creature — Human Soldier
**Status**: PASS

- Mana cost {2}{W}: correct.
- Type Creature, subtypes Human Soldier: correct.
- Power/Toughness 2/2: correct.
- Dies trigger with TriggerKind::SelfDies: correct.
- Targets creature you control (filters by controller): correct.
- Human check gives 2 counters, non-Human gives 1: correct.
- When multiple targets exist, presents a choice to the player: correct.
- When only one target, auto-applies: acceptable simplification.
- Uses `PendingEffect::AddCounters { count: 1, human_bonus: true }` for deferred choice: correct.
- Tests exist in `tier3_cards.rs` (`elder_cathar_grants_counter_on_death`) and `card_mechanics.rs` (`elder_cathar_gives_two_counters_to_human`, `elder_cathar_gives_one_counter_to_non_human`).
