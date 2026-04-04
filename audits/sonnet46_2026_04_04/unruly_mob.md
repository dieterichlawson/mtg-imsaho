## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another creature you control dies, put a +1/+1 counter on this creature.
**Type line**: Creature — Human
**Status**: ISSUE

### Code issues

- Simultaneous death: trigger does not fire when Unruly Mob dies in the same SBA pass as another creature you control.
  - Oracle text says: `Whenever another creature you control dies, put a +1/+1 counter on this creature.`
  - Official ruling says: `If Unruly Mob and another creature you control die simultaneously (perhaps because they were both attacking or blocking), Unruly Mob won't be on the battlefield as its triggered ability resolves. It can't be saved by the +1/+1 counter that would have been put on it.` — this wording confirms the trigger *does* fire, it simply can't save Unruly Mob.
  - Code does: In `mtg-engine/src/triggers.rs` lines 418–419, the DeathWatch watcher scan filters `o.zone == Zone::Battlefield`. By the time `collect_triggers` is called, all creatures killed in the same SBA pass (including Unruly Mob) have already been moved to the graveyard by `destroy()` / `move_object(Zone::Graveyard)` (see `destruction.rs` line 102 and `sba.rs` lines 86–147). Therefore, when processing the other creature's `CreatureDied` event, Unruly Mob is no longer on the battlefield and is not found as a watcher. Its DeathWatch trigger is never collected and never placed on the stack. The practical outcome (no counter placed on Unruly Mob) is the same as the rules-correct behavior, but the trigger should appear on the stack per the ruling.

### Tricky interactions checked

- "another" constraint (Unruly Mob must not trigger for its own death): PASS — `collect_triggers` filters `o.id != dead_id`, and Unruly Mob is also already in the graveyard when its own `CreatureDied` event is processed, so its own death can never result in a DeathWatch trigger for itself.
- "you control" constraint (opponent's creature death must not trigger Unruly Mob): PASS — `on_any_creature_dies` checks `dead_controller == controller` where `controller` is Unruly Mob's current controller; opponent-controlled creature deaths don't add a counter.
- Unruly Mob leaving battlefield between trigger collection and resolution: PASS — `resolve_next_trigger` at `triggers.rs` line 907 re-checks `zone == Zone::Battlefield` before calling `on_any_creature_dies`, and `on_any_creature_dies` itself repeats this check at line 37. If Unruly Mob is removed (e.g., bounced in response), the counter is not placed.
- Simultaneous death (Unruly Mob + another creature you control die in the same SBA pass): ISSUE — see Code issues above. Trigger should fire per ruling but does not.
- Trigger fires one counter per death event (multiple deaths): PASS — each `CreatureDied` event is processed separately in `collect_triggers`, so Unruly Mob accumulates one counter per dying creature in non-simultaneous scenarios.
- "whenever" vs. "when" (trigger fires for every qualifying death, not just the first): PASS — `TriggerKind::AnyCreatureDies` is used in `triggered_abilities`, and `collect_triggers` emits a new `DeathWatch` trigger for each `CreatureDied` event.
- Card data (mana cost {1}{W}, Creature — Human, P/T 1/1, no keywords): PASS — all fields match oracle text exactly.

### Test coverage

- Basic case (another creature you control dies, Unruly Mob gets counter): `tier3_cards.rs:350` (`unruly_mob_gains_counter_when_ally_dies`)
- Same scenario in edge-cases suite: `edge_cases.rs:338` (`death_watch_triggers_fire_on_creature_death`)
- Multiple triggers for same player (Rage Thrower + Unruly Mob both fire when ally dies): `apnap.rs:155` (`same_player_multiple_triggers_all_fire`)
- Opponent's creature dying (should NOT trigger Unruly Mob): NOT TESTED
- Multiple creatures dying in sequence (counter count): NOT TESTED
- Simultaneous death (Unruly Mob + ally die together in same SBA pass, trigger should fire but can't save mob): NOT TESTED
