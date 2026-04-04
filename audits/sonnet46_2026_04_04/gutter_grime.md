## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- Token `is_token` check fails in real gameplay: Gutter Grime incorrectly triggers when a token you control dies
  - Oracle text says: `"Whenever a nontoken creature you control dies"`
  - Code does: `let was_token = state.get_object(dead_id).map(|o| o.is_token).unwrap_or(false);` (`gutter_grime.rs:53`). In real gameplay via `run_game_loop_inner` (`engine.rs:3119-3126`), the SBA loop runs before `process_triggers`. Inside `check_state_based_actions_with_registry`, after moving a dead token to the graveyard (SBA rule 704.5g/h or 704.5f), the very same SBA pass immediately removes token objects from `state.objects` (`sba.rs:307-315`: `state.objects.remove(&id)`). By the time the `DeathWatch` trigger resolves and `on_any_creature_dies` is called, `state.get_object(dead_id)` returns `None`, causing `.unwrap_or(false)` to default to `false`. The `if was_token { return; }` guard never fires, so Gutter Grime incorrectly adds a slime counter and creates an Ooze token for a token death. The root cause is that `PendingTrigger::DeathWatch` (`triggers.rs:20-30`) and `GameEvent::CreatureDied` (`events.rs:31`) do not carry an `is_token` flag — the only source of truth is the object itself, which is gone by resolution time.

### Tricky interactions checked

- **"nontoken" filter survives token cleanup in SBA**: FAIL. See above — `state.objects.remove` in `sba.rs:313` removes dead tokens before `process_triggers` runs (`engine.rs:3121-3123`), causing the is_token check at `gutter_grime.rs:53` to return `None` → `false`.
- **Dynamic P/T continuously updates (ruling 2011-09-22)**: PASS. `effective_power` and `effective_toughness` in `state.rs:851-935` re-read `obj.card_state["pt_source_counter"]` and look up the current slime counter count on the source every time they are called. No snapshot taken.
- **Gutter Grime leaving battlefield makes Ooze tokens 0/0 (ruling 2011-09-22)**: PASS. `move_object` calls `obj.counters.clear()` when any permanent leaves the battlefield (`state.rs:485`). `effective_power/toughness` look up `*src.counters.get(&counter_type).unwrap_or(&0)`, which returns 0 after the counters are cleared. Verified by test `gutter_grime_ooze_tokens_become_zero_without_source`.
- **Multiple Gutter Grimes — each Ooze tracks its own source (ruling 2011-09-22)**: PASS. Each token has `pt_source_counter` set to the specific `self_id` (`gutter_grime.rs:74`), so each Ooze tracks only its creator's slime counters.
- **Only creatures you control trigger it**: PASS. `if dead_controller != controller { return; }` (`gutter_grime.rs:49-51`) correctly filters on `dead_controller` from the `DeathWatch` trigger.
- **Watcher fires while Gutter Grime is on battlefield**: PASS. `on_any_creature_dies` checks `o.zone == Zone::Battlefield` for self before acting (`gutter_grime.rs:44-47`). Also verified in engine: `resolve_next_trigger` checks watcher zone before calling handler (`triggers.rs:908`).
- **Multiple Ooze tokens all grow together (ruling 2011-09-22)**: PASS. Each token stores its source's ObjectId, and `effective_power/toughness` re-reads the counter live. Verified by `gutter_grime_ooze_tokens_grow_with_more_counters`.
- **Parallel Lives — extra Ooze tokens do NOT get dynamic P/T link**: ISSUE (secondary). `create_token_with_subtypes` returns only the primary token's ObjectId (`state.rs:338`). Extra copies created by Parallel Lives (`state.rs:342-345`) never have `pt_source_counter` set (`gutter_grime.rs:73-76`). Extra tokens fall through to `obj.power?` (= 0) in `effective_power/toughness`, making them static 0/0 instead of tracking slime counters. This violates the oracle text's guarantee that created Ooze tokens have the named ability.
- **Token created with correct subtype and color**: PASS. `create_token_with_subtypes("Ooze", ..., vec![Color::Green], vec![CardType::Creature], vec![], vec!["Ooze".into()])` (`gutter_grime.rs:63-69`).
- **Slime counter added before token created, so token P/T = new count**: PASS. `state.add_counters(self_id, CounterType::Slime, 1)` is called before `create_token_with_subtypes` (`gutter_grime.rs:58` vs `gutter_grime.rs:63`). Since dynamic P/T reads counters live, the token immediately reflects the post-increment count.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Nontoken creature triggers (basic case): `gutter_grime.rs:29` (`gutter_grime_creates_dynamic_pt_ooze`) — TESTED
- Dynamic P/T updates as counters accumulate: `gutter_grime.rs:75` (`gutter_grime_ooze_tokens_grow_with_more_counters`) — TESTED
- Token deaths should NOT trigger: `gutter_grime.rs:131` (`gutter_grime_ignores_token_deaths`) — TESTED but DOES NOT catch the real-gameplay bug. The test manually moves the token to graveyard without calling `check_state_based_actions`, so the token object is still in `state.objects` when `process_triggers` runs. The test passes but does not exercise the path where SBA removes the token first.
- Opponent creature deaths should not trigger: `gutter_grime.rs:158` (`gutter_grime_ignores_opponent_deaths`) — TESTED
- Gutter Grime leaving battlefield → Ooze tokens become 0/0: `gutter_grime.rs:182` (`gutter_grime_ooze_tokens_become_zero_without_source`) — TESTED
- Multiple Gutter Grimes each tracking own Ooze tokens: NOT TESTED
- Parallel Lives interaction (extra Ooze tokens should also have dynamic P/T): NOT TESTED
- Token death in full game loop (SBA + triggers integration): NOT TESTED
