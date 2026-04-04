## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another Human creature dies, put a +1/+1 counter on this creature.
**Type line**: Creature — Human
**Status**: ISSUE

### Code issues

- **Missing `o.subtypes` check for Human tokens** (`mtg-engine/src/cards/isd/village_cannibals.rs` lines 39–42)
  - Oracle text says: `Whenever another Human creature dies, put a +1/+1 counter on this creature.`
  - Code does:
    ```rust
    let is_human = state.get_object(dead_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.subtypes.iter().any(|s| s == "Human"))
        .unwrap_or(false);
    ```
    This chain returns `None` (→ `false`) for tokens, because tokens have `card_id: CardId(0)` and `registry.card_data(CardId(0))` returns `None`. The object-level `o.subtypes` field — which is where tokens store their subtypes — is never checked. A Human token dying would not trigger Village Cannibals. The correct pattern, as used in `champion_of_the_parish.rs` lines 46–53 and `engine.rs` lines 2219–2226, is to OR-check both `registry.card_data(o.card_id).subtypes` and `o.subtypes`.

- **Spurious DeathWatch triggers on non-Human deaths** (`mtg-engine/src/triggers.rs` lines 422–441)
  - Oracle text says: `Whenever another Human creature dies` — the trigger condition requires the dying creature to be a Human. Per MTG rules, the trigger only goes on the stack when this condition is met.
  - Code does: The DeathWatch watcher loop (lines 422–441) pushes a `PendingTrigger::DeathWatch` for Village Cannibals on **every** creature death, with no `if !desc.is_empty()` guard. Village Cannibals has `TriggerKind::AnyCreatureDies` in its `triggered_abilities` (with description `"put a +1/+1 counter on Village Cannibals"`), so `trigger_description` returns a non-empty string. A "Village Cannibals's triggered ability (put a +1/+1 counter on Village Cannibals)" entry therefore appears on the stack whenever **any** creature dies — including Zombies, Spirits, Wolves — even though the trigger condition is not met. The Human check is deferred until resolution in `on_any_creature_dies`, at which point the trigger silently does nothing. Compare with the `EnterWatch` dispatch at lines 373–391, which correctly gates on `if !desc.is_empty()`.

- **Simultaneous deaths: Village Cannibals doesn't trigger when it dies alongside a Human** (`mtg-engine/src/triggers.rs` lines 417–441, `mtg-engine/src/sba.rs` lines 53–147)
  - Oracle text says: `Whenever another Human creature dies` — per MTG CR 704.3, simultaneous state-based deaths happen at the same time; a watcher that dies in the same event batch was on the battlefield when those deaths occurred and its ability should trigger.
  - Code does: `check_state_based_actions_with_registry` collects all creatures to destroy into `destroyed_ids`/`zero_toughness_ids` vectors, then processes them **sequentially**, moving each one to the graveyard with `move_object` before moving on to the next. `collect_triggers` runs only after all zone changes complete. At that point, the DeathWatch watcher scan (`state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`) no longer finds Village Cannibals because it is already in `Zone::Graveyard`. If a Human creature and Village Cannibals die in the same SBA pass (e.g., both take lethal combat damage), Village Cannibals receives no counter for the Human's death.

### Tricky interactions checked

- **"another" exclusion (Village Cannibals watching its own death)**: PASS — `collect_triggers` filters `o.id != dead_id`, so Village Cannibals is never added as a watcher for its own death.
- **Village Cannibals leaving battlefield before trigger resolves**: PASS — `resolve_next_trigger` re-checks `o.zone == Zone::Battlefield` before calling `on_any_creature_dies`, and the card itself also checks this on entry. Both guards are redundant but correct.
- **Human token subtype check**: FAIL — as documented above; `o.subtypes` is not consulted.
- **Trigger fires for every creature death, not just Humans**: FAIL — as documented above; DeathWatch dispatch lacks the `!desc.is_empty()` guard.
- **Simultaneous deaths with Village Cannibals**: FAIL — as documented above; collect_triggers scans post-zone-change battlefield.
- **Non-Human creature dying (functional result)**: PASS — `on_any_creature_dies` correctly returns without adding counters when the dead creature's registry data shows it is not a Human.
- **Card data correctness (mana cost {2}{B}, 2/2, Human, no keywords)**: PASS — matches oracle text.
- **triggered_abilities declaration (AnyCreatureDies registered)**: PASS — `triggered_abilities` includes `TriggerKind::AnyCreatureDies`, enabling the dispatch to find Village Cannibals as a watcher.
- **Counter type (+1/+1)**: PASS — `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)` is correct.

### Test coverage

- Human creature on opponent's side dying triggers counter: `tier3_cards.rs:431` TESTED
- Non-Human creature dying does not trigger counter: `tier3_cards.rs:458` TESTED — but the test only asserts counter count = 0; it does not verify that no trigger was placed on the stack, so the spurious-trigger bug is not caught.
- Human token dying triggers counter: NOT TESTED
- Village Cannibals and a Human die simultaneously (same SBA pass): NOT TESTED
- Village Cannibals leaves battlefield before trigger resolves: NOT TESTED
