## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Status**: ISSUE

### Code issues

- ETB trigger suppressed when source leaves battlefield before resolution (`mtg-engine/src/triggers.rs`, lines 893–899)
  - Oracle text says: `When this creature enters, mill four cards.`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { if let Some(behavior) = registry.get(card_id) { behavior.on_enter_battlefield(state, object_id, registry); } }` — if Armored Skaab leaves the battlefield (e.g., is bounced in response to its ETB trigger) between when the trigger is collected and when it resolves, the mill never happens. Per MTG rules CR 603.6c, once a triggered ability is on the stack it exists independently of its source; the source leaving the battlefield does not prevent the trigger from resolving. The mill should happen regardless.

### Tricky interactions checked

- Fewer-than-four cards in library: PASS — `mill_cards` breaks out of its loop when `library_order.is_empty()`, correctly putting all remaining cards into the graveyard and stopping (matches the Scryfall ruling from 2011-09-22).
- Mill targets the controller (not the opponent): PASS — `on_enter_battlefield` reads the controller via `state.get_object(object_id).map(|o| o.controller)` and passes that to `mill_cards`. The controller is correctly identified at resolution time.
- ETB trigger fires at all: PASS — `collect_triggers` (triggers.rs lines 344–363) correctly creates a `PendingTrigger::EnteredBattlefield` entry on `GameEvent::EnteredBattlefield`, and `resolve_next_trigger` calls `on_enter_battlefield`, which calls `mill_cards(state, controller, 4)`.
- Source leaves battlefield before ETB trigger resolves: FAIL — see Code Issues above. The battlefield-zone guard at resolution prevents the mill from happening, contrary to CR 603.6c.
- Log message accuracy when library has fewer than 4 cards: The `on_enter_battlefield` handler always logs "Armored Skaab enters — milled 4 cards" even when 0–3 cards were actually milled (cosmetic inaccuracy, not a gameplay bug). `mill_cards` itself logs the actual count separately.

### Test coverage

- Basic ETB mills 4 cards from controller's library: NOT TESTED
- ETB with fewer than 4 cards in library mills only available cards: NOT TESTED
- ETB trigger resolves even if source leaves battlefield before resolution: NOT TESTED
