## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
**Type line**: Creature — Treefolk
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Mana cost, types, subtypes, P/T**: `{2}{G}{G}`, Creature, Treefolk, 1/1 — all match oracle text exactly. PASS.
- **Hexproof keyword present in `keywords` vec**: `keywords: vec![Keyword::Hexproof]` declared. PASS.
- **Hexproof enforced at targeting**: `engine.rs:758–767` `can_be_targeted` calls `state.has_keyword(target_id, Keyword::Hexproof, registry)` and returns false if the controller is an opponent. PASS.
- **`has_keyword` checks both registry and runtime object**: `state.rs:987–1043` checks `obj.keywords` (runtime), then card definition, then continuous effects, then temporary grants. Lumberknot's hexproof is in the card definition and is found correctly. PASS.
- **TriggerKind::AnyCreatureDies declared**: `triggered_abilities` contains `TriggerKind::AnyCreatureDies`. PASS.
- **Death-watch dispatch in `triggers.rs`**: `collect_triggers` (lines 418–441) processes `GameEvent::CreatureDied`, collects all battlefield permanents except the dead creature, and creates a `DeathWatch` trigger for each registered permanent. Lumberknot is included as a watcher whenever any other creature dies. PASS.
- **`on_any_creature_dies` handler**: Adds one `PlusOnePlusOne` counter via `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)`. PASS.
- **Zone guard before adding counter**: Handler checks `state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before placing counter — correctly prevents placing a counter on Lumberknot if it left the battlefield after the trigger was created but before it resolved. PASS.
- **Sacrifice causes death trigger**: `sacrifice` in `destruction.rs` calls `destroy`, which pushes `GameEvent::CreatureDied` and moves to graveyard — Lumberknot's trigger correctly fires on sacrificed creatures. PASS.
- **Multiple simultaneous deaths (board wipe)**: SBA in `sba.rs` identifies all dying creatures in one batch, then processes them sequentially (event pushed + move to graveyard for each). `collect_triggers` processes each `CreatureDied` event separately; Lumberknot is on the battlefield for each event (it's not dying), so it correctly accumulates one trigger per death. PASS.
- **Lumberknot itself dying**: The death-watch watcher filter `o.id != dead_id` explicitly excludes the dying creature. If Lumberknot dies, its own "whenever a creature dies" ability does not fire. Per strict MTG rules the trigger should fire and fizzle (Lumberknot can't receive a counter in the graveyard), but since the end game state is identical — no counter placed — this has zero impact on play. PASS (no observable difference).
- **Engine DeathWatch vs. ETB-watch dispatch inconsistency (noted, not an issue for Lumberknot)**: The ETB-watch block guards with `if !desc.is_empty()` before adding a trigger (`triggers.rs:375`). The death-watch block does NOT (`triggers.rs:424–438`): it creates a `DeathWatch` trigger for every registered permanent on the battlefield regardless of whether `desc` is empty. This causes spurious no-op triggers on the stack for permanents without `AnyCreatureDies`. This engine behavior does not cause Lumberknot to behave incorrectly — Lumberknot's trigger is still correctly created and resolved — but it is worth noting.
- **"whenever" per-death semantics**: The oracle says "whenever a creature dies" — one trigger per creature death. The engine fires one `DeathWatch` per `CreatureDied` event. PASS.
- **"this creature" targeting (self-reference)**: `on_any_creature_dies` uses `self_id` (Lumberknot's own ID) as the counter target. PASS.

### Test coverage
- Basic trigger: any creature dies, Lumberknot gains +1/+1 counter: `tier3_cards.rs:377` (`lumberknot_gains_counter_on_any_death`) — TESTED.
- Simultaneous deaths (board wipe, multiple counters): NOT TESTED.
- Lumberknot's own death (trigger does not fire / no counter placed): NOT TESTED.
- Hexproof prevents opponent from targeting Lumberknot specifically: NOT TESTED (hexproof targeting correctness is tested generically via `witchbane_orb.rs` and `innistrad_cards.rs`).
- Trigger does not fire when Lumberknot is off battlefield at resolution: NOT TESTED.
