## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature or another creature dies, target player mills a card.
**Type line**: Creature — Human Rogue
**Status**: ISSUE

### Code issues

- Selhoff Occultist's `AnyCreatureDies` trigger does not fire for creatures that die simultaneously with it (e.g., in a board wipe).
  - Oracle text says: `"Whenever this creature or another creature dies, target player mills a card."`
  - Code does: The death-watch watcher scan in `mtg-engine/src/triggers.rs:418` filters watchers with `o.zone == Zone::Battlefield`. By the time `collect_triggers` runs after SBA processing, the Occultist has already been moved to the graveyard (via `state.move_object(id, Zone::Graveyard)` in `destruction.rs:102`). So when multiple creatures die in the same SBA pass, the Occultist is absent from the battlefield watcher scan for every `CreatureDied` event it did not itself generate. Only the `SelfDies` trigger fires (1 mill). In a board wipe killing the Occultist and N other creatures, the card should produce N+1 mills but only produces 1.

### Tricky interactions checked

- **Occultist's own death (`SelfDies` path)**: Correct. The `SelfDies` trigger fires regardless of zone. `on_dies` retrieves the controller via `state.get_object(object_id)`, which reads from the objects HashMap regardless of zone, so the graveyard object is found. PASS.
- **Another creature dies alone (non-simultaneous, Occultist alive)**: Correct. The death-watch watcher scan finds the Occultist on the battlefield, adds a `DeathWatch` trigger, `resolve_next_trigger` verifies it's still on the battlefield, `on_any_creature_dies` checks zone again, and calls `present_mill_choice`. PASS.
- **Simultaneous death — Occultist dies with other creatures**: FAIL. As described above, the Occultist is off the battlefield when the watcher scan runs, so `AnyCreatureDies` triggers for co-dying creatures are never collected.
- **"target player" — any player can be targeted**: Correct. `present_mill_choice` builds options from `state.players.iter()` covering all players, not just the opponent. PASS.
- **Mandatory targeting (`optional: false`)**: Correct. Oracle text has no "may," and the code sets `optional: false`. PASS.
- **Mill count**: Correct. `PendingEffect::Mill { count: 1 }` applies 1 mill per trigger. PASS.
- **`on_any_creature_dies` zone guard during resolution**: The trigger resolution in `triggers.rs:908` checks `o.zone == Zone::Battlefield` before calling `on_any_creature_dies`. This would suppress a legitimately queued `DeathWatch` trigger if the Occultist died after the trigger was collected but before it resolved. Per MTG rules (CR 603.10), "target player mills a card" has no battlefield-presence requirement on the source. However, given the engine resolves all triggers synchronously before giving priority, this scenario cannot occur in practice with the current game loop architecture — the Occultist cannot be destroyed in response to its own queued trigger. PASS (no practical impact under current engine).
- **Double-triggering prevention**: When the Occultist dies, the death-watch watcher scan uses `o.id != dead_id` to exclude the dead creature itself, so `AnyCreatureDies` does not fire a second time for the Occultist's own death alongside the `SelfDies` path. PASS.
- **`SelfDies` trigger description lookup**: `trigger_description` looks up `TriggerKind::SelfDies` in the card's `triggered_abilities` and finds `"target player mills a card"`. Correct. PASS.
- **Mill effect resolves correctly**: `mill_cards` in `engine.rs:2755` moves cards from the library to the graveyard (moves object, not just removes card_id). Per MTG, milling a player with an empty library is not an error — they lose at SBA. The code breaks without error. PASS.

### Test coverage

- Occultist self-death triggers mill: NOT TESTED (no test file found referencing selhoff, occultist, or Selhoff Occultist)
- Another creature dying alone triggers Occultist mill: NOT TESTED
- Simultaneous death — Occultist + other creatures die together, should produce multiple mills: NOT TESTED
- "Target player" includes controller (can mill self): NOT TESTED
- Mill with empty library does not crash: NOT TESTED
