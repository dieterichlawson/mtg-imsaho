## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
**Type line**: Creature — Vampire Warrior
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Trigger only fires on combat damage, not non-combat damage**: PASS. The card declares `TriggerKind::CombatDamageToPlayer` and the engine only queues it from `GameEvent::CombatDamageDealt` (lines 459–564 of `triggers.rs`), never from `GameEvent::NonCombatDamageDealt`.
- **Trigger fires for damage to any player** (not just the opponent): PASS. The dispatch fires for any `DamageTarget::Player(player_id)` regardless of which player was damaged — consistent with oracle text "a player."
- **Counter count is exactly two**: PASS. `state.add_counters(self_id, CounterType::PlusOnePlusOne, 2)` adds exactly 2 counters as the oracle text requires.
- **No "may" optionality**: PASS. The counters are placed automatically with no optional choice, which is correct — the oracle text does not say "you may."
- **Creature leaves battlefield before trigger resolves**: PASS. `resolve_next_trigger` does not guard `CombatDamageToPlayer` with a battlefield check, but the card handler does: `if state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`. If the creature dies in the same SBA window (e.g., while blocked), the trigger resolves but correctly places no counters, consistent with MTG rules.
- **Flying and Haste keywords declared**: PASS. `keywords: vec![Keyword::Flying, Keyword::Haste]` matches the oracle text.
- **Mana cost {3}{R}{R}**: PASS. `Generic(3)` + two `Colored(Red)` matches exactly.
- **P/T 2/2**: PASS. `power: Some(2), toughness: Some(2)`.
- **Subtypes Vampire Warrior**: PASS. `subtypes: vec!["Vampire".into(), "Warrior".into()]`.
- **Trigger description non-empty so dispatch fires**: PASS. `triggered_abilities` declares `CombatDamageToPlayer` with description "put two +1/+1 counters on Falkenrath Marauders"; `trigger_description()` returns this non-empty string, satisfying the `!desc.is_empty()` guard at `triggers.rs:499` that gates trigger collection.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Two counters placed on combat damage to player: `mtg-engine/tests/tier6_cards.rs:307` (`falkenrath_marauders_two_counters_on_combat_damage`)
- Trigger does not fire on non-combat damage: NOT TESTED
- Trigger fires when damaging any player (not just opponent): NOT TESTED
- Creature leaves battlefield before trigger resolves (no counters added): NOT TESTED
