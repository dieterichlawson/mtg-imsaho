## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues

- **Engine bug: DeathWatch trigger never collected when Ghoul and victim die simultaneously** (`mtg-engine/src/triggers.rs` lines 418–419)
  - Oracle text says: `"Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness."` — no condition requiring the Ghoul to be on the battlefield at trigger time.
  - Code does: `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)` — watcher scan runs inside `collect_triggers`, which is called AFTER all SBA deaths have been applied. When the Ghoul and a creature it damaged die in the same SBA round (e.g., Blasphemous Act board wipe, or mutual lethal first-strike damage from two first-strikers), the Ghoul has already been moved to `Zone::Graveyard` by the time the victim's `CreatureDied` event is processed in `collect_triggers`. The Ghoul is not found as a watcher, so the DeathWatch trigger is never pushed onto the stack, and no life is gained. The oracle text contains no "as long as this creature is on the battlefield" condition.

- **Engine bug: DeathWatch trigger cancelled at resolution if Ghoul left battlefield** (`mtg-engine/src/triggers.rs` lines 906–912)
  - Oracle text says: `"Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness."` — no intervening-if clause.
  - Code does: `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_any_creature_dies(...) }` — if the trigger IS collected (Ghoul was on battlefield when `collect_triggers` ran) but the Ghoul then leaves the battlefield before the trigger resolves (e.g., destroyed in response), the engine cancels the trigger. Per MTG rules, triggered abilities without an intervening-if clause resolve regardless of the source's location. The life-gain effect makes no reference to the Ghoul's presence.

### Tricky interactions checked

- **Simultaneous deaths from board wipe (e.g., Blasphemous Act kills Ghoul + creatures it damaged)**: FAIL — all SBA deaths are committed before `collect_triggers` runs; Ghoul is gone from the battlefield when the victim's `CreatureDied` event is scanned, so the trigger is never collected.
- **Mutual first-strike deaths (Ghoul blocks a first-striker that deals ≥2 damage)**: FAIL — same root cause as board wipe; both die in the same SBA round; trigger not collected.
- **Normal first-strike scenario (Ghoul kills non-first-striker in first-strike step)**: PASS — Ghoul is still on the battlefield when SBA fires for the victim; trigger is correctly collected and resolves.
- **Last-known toughness after -X/-X before regular combat step (ruling scenario)**: PASS — `destroy()` and the zero-toughness path in `sba.rs` both call `effective_toughness` before `state.move_object`, correctly capturing post-modification toughness.
- **No life gain if Ghoul didn't damage the creature**: PASS — `dead_damaged_by.contains(&self_id)` check in `on_any_creature_dies` is correct.
- **`damaged_by` reset at end of turn ("this turn" constraint)**: PASS — cleanup step in `engine.rs` clears `damaged_by` (together with `damage_marked`) for all battlefield creatures with `damage_marked > 0`; regeneration and zone-change paths also clear it.
- **Life gain of 0 when creature dies at 0 toughness**: PASS — `dead_toughness.max(0)` + `if toughness > 0` guard correctly skips a zero-life-gain event.
- **`damaged_by` tracking for combat vs. non-combat damage**: PASS — combat uses dedup push (`if !obj.damaged_by.contains(&source)`); non-combat uses plain push but `contains` check in trigger handler is unaffected by duplicates.
- **Trigger cancellation after Ghoul destroyed in response**: FAIL — `resolve_next_trigger` in `triggers.rs` re-checks `watcher_id` zone; Abattoir Ghoul's ability has no intervening-if clause so should resolve regardless.
- **Card data (cost, types, keywords, P/T)**: PASS — {3}{B}, Creature — Zombie, 3/2, First Strike all match oracle.

### Test coverage

- Basic life gain when Ghoul is alive and victim dies: `tier6_cards.rs:20` (`abattoir_ghoul_gains_life_from_damaged_creature_death`)
- No life gain when Ghoul did not damage the creature: `tier6_cards.rs:43` (`abattoir_ghoul_no_life_if_not_damaged_by_ghoul`)
- Last-known toughness respects +1/+1 counters: `tier6_cards.rs:61` (`abattoir_ghoul_uses_last_known_toughness_with_counters`)
- Simultaneous death (board wipe / mutual first-strike): NOT TESTED
- Trigger resolves after Ghoul destroyed in response: NOT TESTED
- Ruling scenario (first-strike damage then -5/-5 before regular step): NOT TESTED
- `damaged_by` cleared at end of turn (no cross-turn firing): NOT TESTED
