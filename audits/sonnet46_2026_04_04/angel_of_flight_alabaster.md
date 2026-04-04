## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Type line**: Creature — Angel
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **"your upkeep" — trigger fires only for controller's upkeep**: The trigger collection in `triggers.rs` (lines 597–642) fires `UpkeepTrigger` for ALL permanents on the battlefield whenever ANY upkeep starts, not filtered to the active player's permanents. However, the `on_upkeep` handler in the card (`angel_of_flight_alabaster.rs` lines 43–45) guards with `if state.active_player != controller { return; }`. Because `process_triggers` resolves all triggers synchronously before players receive priority, this phantom trigger during the opponent's upkeep is entirely invisible to players and produces no game-state changes. Functionally correct.

- **No Spirit in graveyard (ruling: ability goes on stack and is removed with no effect)**: `present_target_choice` in `helpers.rs` line 126 returns immediately when `targets.is_empty()`. The `UpkeepTrigger` was already popped from the stack by `resolve_next_trigger` before `on_upkeep` is called, so the ability is silently removed from the stack with no effect. Matches the ruling. Pass.

- **Exactly 1 Spirit in graveyard (mandatory, auto-applies)**: `present_target_choice` with `optional: false` and `targets.len() == 1` auto-applies the effect (lines 129–133 of `helpers.rs`). Correct — the player has no real choice when exactly one legal target exists and the ability is mandatory.

- **Multiple Spirits in graveyard (player must choose one)**: `present_target_choice` sets `state.awaiting_action = Some(AwaitingAction::ResolutionChoice { ... })` with the list of spirit targets. Correctly presents a mandatory (non-optional) target choice.

- **Spirit token subtype check (tokens store subtypes on object, not registry)**: The filter at lines 50–53 checks both `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Spirit"))` AND `o.subtypes.iter().any(|s| s == "Spirit")`. Correctly handles both registry-backed cards and tokens. Pass.

- **"your graveyard" — owner vs controller**: `objects_in_zone(Zone::Graveyard, controller)` is called with the Angel's controller. Per `state.rs` line 603, graveyard zone filters by `obj.owner == player`. This correctly restricts targets to cards owned by the Angel's controller, matching "your graveyard." Pass.

- **Mandatory vs "you may"**: Oracle has no "you may." Code uses `optional: false` in `present_target_choice`. Pass.

- **ReturnToHand moves to correct zone**: `apply_pending_effect` at `engine.rs` line 2303 calls `state.move_object(*id, Zone::Hand)`. Since all targeted Spirits were filtered by owner (the Angel controller), the object moves to Zone::Hand under that owner's hand. Pass.

- **Angel must be on battlefield for trigger to fire**: Both the collection side (triggers.rs only scans `o.zone == Zone::Battlefield`) and the resolution side (`state.get_object(object_id).map(|o| o.zone == Zone::Battlefield)` check in `resolve_next_trigger` line 955) verify this. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Single Spirit in GY returns to hand on upkeep: `tier7_cards.rs:225` — TESTED
- No Spirit in GY → ability fizzles with no effect: NOT TESTED
- Multiple Spirits in GY → choice presented to controller: NOT TESTED
- During opponent's upkeep → trigger does not fire: NOT TESTED
- Spirit subtype check covers tokens (object subtypes): NOT TESTED
- "Your graveyard" owner filter: NOT TESTED
