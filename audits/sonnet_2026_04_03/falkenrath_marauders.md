## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
**Type line**: Creature — Vampire Warrior
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Combat damage to player trigger: PASS — Uses TriggerKind::CombatDamageToPlayer, dispatched correctly in triggers.rs lines 489-514, resolved in lines 921-924
- Counter addition amount: PASS — Correctly adds exactly 2 +1/+1 counters via state.add_counters(self_id, CounterType::PlusOnePlusOne, 2)
- Source leaves battlefield before resolution: PASS — Code properly checks if source is still on battlefield (zone == Zone::Battlefield) before adding counters
- Damage to planeswalker exclusion: PASS — Trigger only fires on damage to Player, not planeswalker, matching oracle text and research findings

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Combat damage to player gets 2 +1/+1 counters: `tier6_cards.rs:307` (falkenrath_marauders_two_counters_on_combat_damage)
- Damage to planeswalker does not trigger: NOT TESTED
- Source leaving battlefield before resolution: NOT TESTED
- Non-combat damage does not trigger: NOT TESTED