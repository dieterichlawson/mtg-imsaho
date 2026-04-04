## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, target creature can't block this turn.
**Type line**: Creature — Vampire
**Status**: ISSUE

### Code issues

- ETB trigger is suppressed if Crossway Vampire leaves the battlefield before the trigger resolves.
  - Oracle text says: `When this creature enters, target creature can't block this turn.`
  - Code does: `mtg-engine/src/triggers.rs:893-900` — `resolve_next_trigger` for `PendingTrigger::EnteredBattlefield` contains `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` guard. If Crossway Vampire is no longer on the battlefield when the trigger resolves (e.g., destroyed by another trigger that resolves in the same batch before this one), `on_enter_battlefield` is skipped and the "can't block this turn" effect never applies. The oracle text's ability does not reference the source and should resolve regardless of whether the source is on the battlefield at resolution time.

### Tricky interactions checked

- **"target creature" with no "another" qualifier**: `creature_targets(state)` at `cards/helpers.rs:166-171` includes ALL creatures on the battlefield including Crossway Vampire itself. This is correct — the oracle text does not say "another target creature." PASS
- **Mandatory targeting (not "you may")**: `present_target_choice` is called with `optional: false`, making the effect mandatory. Correct per oracle text which has no "you may." PASS
- **Auto-apply with exactly 1 legal target**: `present_target_choice` at `cards/helpers.rs:129-133` auto-applies the effect when there is exactly 1 target and `optional == false`. Correct behavior. PASS
- **No valid targets at resolution (empty battlefield edge case)**: If no creatures are on the battlefield when the trigger resolves, `present_target_choice` returns early (does nothing). This is functionally correct per MTG rules (triggered ability with no legal targets fizzles). PASS
- **"can't block this turn" scoped to current turn**: Effect pushes to `state.until_end_of_turn_cant_block` (engine.rs:2248). This list is cleared at the cleanup step (`engine.rs:3023`). PASS
- **Enforcement in combat**: `combat::eligible_blockers` at `combat.rs:611` explicitly checks `!state.until_end_of_turn_cant_block.contains(&id)`, correctly preventing affected creatures from blocking. PASS
- **ETB trigger dispatch**: `collect_triggers` in `triggers.rs:344-392` correctly processes `GameEvent::EnteredBattlefield` and creates `PendingTrigger::EnteredBattlefield` for the card. The trigger is dispatched and `on_enter_battlefield` is called. PASS (under normal conditions, but see issue above)
- **ETB trigger fizzles if source leaves battlefield before resolution**: `triggers.rs:895` guards `on_enter_battlefield` with a battlefield-presence check. For abilities that don't reference the source, MTG rules (CR 603.6) require the trigger to resolve regardless. FAIL — see Code issues above.
- **`CantBlockThisTurn` effect correctly applied**: `apply_pending_effect` at `engine.rs:2246-2249` handles `(Target::Object, PendingEffect::CantBlockThisTurn)` by pushing the object ID to `until_end_of_turn_cant_block` and logging the event. PASS
- **Redundant blocking-check in `can_block`**: `state.can_block()` (`state.rs:962-983`) does NOT check `until_end_of_turn_cant_block`. However, `eligible_blockers` in `combat.rs` makes a separate explicit check at line 611, so enforcement is complete. PASS (not a bug, just redundant architecture)

### Test coverage

- ETB trigger fires and "can't block this turn" is applied: NOT TESTED (no direct test for Crossway Vampire)
- "can't block this turn" enforcement in `eligible_blockers`: `card_mechanics.rs:175-188` (via Nightbird's Clutches, same mechanic)
- `until_end_of_turn_cant_block` cleared at end of turn: NOT TESTED for Crossway Vampire
- Mandatory targeting (no "you may"): NOT TESTED
- Targeting self (no "another" restriction): NOT TESTED
- ETB trigger suppressed when source leaves before resolution: NOT TESTED
