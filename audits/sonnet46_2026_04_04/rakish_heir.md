## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Self-triggering (Rakish Heir itself attacks)**: pass. In `collect_triggers`, the watcher scan for `CombatDamageDealt → DamageTarget::Player` uses `.filter(|o| o.zone == Zone::Battlefield)` with no self-exclusion (triggers.rs:518–521). A `CombatDamageWatch` trigger is collected with `watcher_id = heir_id`, `source_id = heir_id`. The handler checks source is a Vampire controlled by Rakish Heir's controller — passes — and puts the counter on `source_id` (Rakish Heir itself). Correct.
- **Vampire token triggering**: pass. The subtype check at rakish_heir.rs:48–51 reads BOTH `registry.card_data(source.card_id)` and `source.subtypes`, so Vampire tokens (which carry subtypes on the object rather than in registry data) are correctly recognized.
- **Opponent-controlled Vampire**: pass. The handler at rakish_heir.rs:44–46 requires `o.controller == controller` (Rakish Heir's controller). An opponent's Vampire has a different controller, so the match arm returns early. No counter is awarded.
- **Multiple Vampires attacking simultaneously**: pass. Each attacker generates a separate `CombatDamageDealt` event; `collect_triggers` iterates over all events since `trigger_event_index`, producing one `CombatDamageWatch` trigger per event. Each Vampire that dealt player damage gets a separate counter.
- **Non-Vampire creature dealing player damage**: pass. The handler checks `is_vampire` (both registry and runtime subtypes) and returns without awarding a counter if false. Confirmed by the `rakish_heir_no_counter_on_non_vampire` test.
- **Rakish Heir dies from first-strike damage in the same combat**: pass. `deal_combat_damage` runs SBAs between first-strike and normal damage steps (combat.rs:147). By the time normal-damage `CombatDamageDealt` events are processed in `collect_triggers`, Rakish Heir is already off the battlefield and is excluded from the watcher scan. Per MTG rules this is correct: Rakish Heir is no longer in play when the trigger event occurs, so its ability does not trigger.
- **Counter placed on "it" (the Vampire that dealt damage), not on Rakish Heir**: pass. `state.add_counters(source_id, …)` at rakish_heir.rs:54 targets the creature that dealt damage, matching "put a +1/+1 counter on it."
- **"You control" at trigger resolution vs. controller change**: pass for typical cases. The handler re-reads Rakish Heir's controller at resolution time (rakish_heir.rs:39–42); if Rakish Heir has been stolen since the trigger fired, the wrong controller would be used. However, this is an engine-wide limitation affecting many watching triggers, not a Rakish Heir-specific bug, and stealing Rakish Heir between trigger queue and resolution in the same batch is not possible with the current synchronous `process_triggers` model.
- **Trigger uses `CombatDamageWatch` path (not `CombatDamageToPlayer`)**: pass. Rakish Heir registers `TriggerKind::AnyCombatDamageToPlayer`, not `CombatDamageToPlayer`. `collect_triggers` only queues a `CombatDamageToPlayer` trigger if the source's own description is non-empty for that kind (triggers.rs:498–514); Rakish Heir has none. Rakish Heir is correctly picked up as a watcher (triggers.rs:524–540), and `resolve_next_trigger` dispatches `CombatDamageWatch` to `on_any_combat_damage_to_player` (triggers.rs:933–939). Correct.
- **"combat damage" vs. non-combat damage**: pass. The `AnyCombatDamageToPlayer` watcher is only collected from `GameEvent::CombatDamageDealt` (triggers.rs:459). Non-combat damage fires `GameEvent::NonCombatDamageDealt` (triggers.rs:566), which only populates `DamageToPlayerWatch` triggers — Rakish Heir does not register `AnyDamageToPlayer`. A burn spell would not trigger Rakish Heir.

### Test coverage
- Self-counter on combat damage: `tier6_cards.rs:200` (`rakish_heir_self_counter_on_combat_damage`) — TESTED
- Counter on other controlled Vampire dealing combat damage: `tier6_cards.rs:220` (`rakish_heir_counter_on_other_vampire_combat_damage`) — TESTED
- No counter on non-Vampire: `tier6_cards.rs:242` (`rakish_heir_no_counter_on_non_vampire`) — TESTED
- No counter on opponent-controlled Vampire: NOT TESTED
- Vampire token triggering: NOT TESTED
- Multiple simultaneous Vampires each get a counter: NOT TESTED
- No counter on non-combat damage to player: NOT TESTED
