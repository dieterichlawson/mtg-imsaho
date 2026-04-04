## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

- ETB trigger silently dropped when Fiend Hunter has left the battlefield before resolution
  - Oracle text says: `"When this creature enters, you may exile another target creature."`
  - Official ruling says: `"If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the target creature indefinitely."`
  - Code does: In `mtg-engine/src/triggers.rs` lines 893–899, the `EnteredBattlefield` trigger resolution has a guard `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` that skips calling `on_enter_battlefield` when the source is no longer on the battlefield. If an opponent kills or bounces the Fiend Hunter in response to its ETB trigger, the ETB trigger silently does nothing — the exile never happens. Per the ruling, the ETB trigger should still resolve and exile the target creature (permanently, since the LTB trigger already fired and did nothing). This is a general engine bug: triggered abilities do not need their source to still be on the battlefield to resolve.

### Tricky interactions checked

- **"you may" optionality**: PASS — `present_optional_target_choice` (helpers.rs:148–157) correctly passes `optional=true` to `present_target_choice`, which always presents a `ChooseTarget` choice to the player. `apply_pending_effect` is only called when the player picks a non-None target (engine.rs:2005–2007).
- **"another" exclusion**: PASS — `creature_targets_except(state, object_id)` (helpers.rs:174–179) correctly excludes Fiend Hunter itself from the valid target list.
- **"under its owner's control"**: PASS — `on_leave_battlefield` in fiend_hunter.rs line 65 explicitly sets `obj.controller = obj.owner` after returning the exiled creature.
- **ETB trigger fires when FH leaves before resolution**: FAIL — see Code Issues above. The engine's `resolve_next_trigger` (triggers.rs:893–899) gates ETB trigger resolution on `o.zone == Zone::Battlefield`. If FH leaves in response to its own ETB trigger, the exile never happens, contradicting the ruling.
- **LTB trigger fires and returns creature (normal case)**: PASS — `GameEvent::LeftBattlefield` dispatches a `PendingTrigger::LeftBattlefield` (triggers.rs:443–458); resolution (triggers.rs:975–978) calls `on_leave_battlefield` without any zone check on the source (correct: LTB triggers need to fire after the source has already left). `card_state` is not cleared by `move_object` (state.rs:479–487), so the `"exiled_creature"` key survives the zone change.
- **Token exiled — not returned**: PASS — SBA (sba.rs:307–315) removes tokens from `state.objects` once they are no longer on the battlefield (rule 704.5d). When `on_leave_battlefield` calls `state.get_object(target_id)` for a token that has ceased to exist, it gets `None`, which causes `unwrap_or(false)` to return `false`, so no return attempt is made.
- **No valid targets**: PASS — `present_optional_target_choice` delegates to `present_target_choice` (helpers.rs:117–145), which returns early when `targets.is_empty()` without setting `awaiting_action`. Correct: if no other creatures exist, the trigger fizzles harmlessly.
- **"exiled card" (not token) in LTB text**: PASS — tokens cease to exist (see token check above), so the return-to-battlefield only fires for real (non-token) cards, which is consistent with the ruling.
- **Exiled creature already returned by other means**: PASS — `on_leave_battlefield` checks `o.zone == Zone::Exile` before returning; if the creature has already left exile, the code does nothing.
- **Mana cost / P/T / types / subtypes**: PASS — card_data declares `{1}{W}{W}`, 1/3, `Creature`, `["Human", "Cleric"]`, matching the Scryfall oracle.

### Test coverage

- Normal ETB exile (FH on battlefield, chooses to exile an opponent creature): `tier3_cards.rs:211` (`fiend_hunter_exiles_on_etb`) — TESTED
- LTB return (FH dies after successful exile): `card_mechanics.rs:127` (`fiend_hunter_returns_exiled_on_death`) — TESTED
- "you may" optionality / own-creature targeting: `card_fixes.rs:30` (`fiend_hunter_can_target_own_creature`) — TESTED
- Choice presented with multiple targets: `card_fixes.rs:60` (`fiend_hunter_presents_choice_with_multiple_targets`) — TESTED
- **Ruling: FH leaves before ETB resolves → LTB fires first (does nothing) → ETB resolves and exiles indefinitely**: NOT TESTED
- **Ruling: token exiled → not returned when FH leaves**: NOT TESTED
- **Ruling: FH owner loses game → exiled creature remains exiled**: NOT TESTED
- "you may" declined (player opts out when targets exist): NOT TESTED
