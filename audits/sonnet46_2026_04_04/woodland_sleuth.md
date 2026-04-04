## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
**Type line**: Creature — Human Scout
**Status**: ISSUE

### Code issues

- **Intervening-if condition not checked at trigger-collection time** (`mtg-engine/src/triggers.rs` lines 344–363)
  - Oracle text says: `"When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand."`
  - Code does: `if registry.get(card_id).is_some() { ... ap_triggers.push(trigger); ... }` — the ETB trigger is unconditionally pushed onto the stack whenever Woodland Sleuth enters the battlefield, with no check of `state.creature_died_this_turn`. Per CR 603.4 (intervening-if clause), the trigger should only go on the stack when the "if" condition is true at the time of the triggering event. When no creature has died this turn the trigger appears on the stack and grants both players a spurious priority window, even though it will do nothing on resolution.

- **Woodland Sleuth cannot be returned to its own hand when it dies in response to its ETB trigger** — two bugs, both must be fixed:
  1. Engine guard in `mtg-engine/src/triggers.rs` lines 893–899:
     - Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`
     - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — the trigger resolution is entirely skipped when the Sleuth is no longer on the battlefield, so the ability never fires if the Sleuth was killed in response to its own trigger.
  2. Card-level guard in `mtg-engine/src/cards/isd/woodland_sleuth.rs` lines 45–48:
     - Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`
     - Code does: `let controller = match state.get_object(object_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` — even if the engine's check were fixed, the card itself early-returns when the source is not on the battlefield, preventing it from ever scanning the graveyard or returning itself.

### Tricky interactions checked

- **Intervening-if clause (CR 603.4) — trigger only goes on stack when condition is true**: FAIL — the trigger is collected unconditionally in `collect_triggers`; `creature_died_this_turn` is never tested before adding the `EnteredBattlefield` pending trigger to the stack.
- **Woodland Sleuth dies in response to its own ETB trigger — trigger still resolves**: FAIL — two independent checks (engine: `zone == Battlefield` guard; card: `zone == Battlefield` controller lookup guard) both prevent the trigger from resolving when Sleuth is in the graveyard.
- **Random selection deferred until resolution** (ruling: "The creature card isn't chosen at random until the ability resolves."): PASS — the shuffle happens inside `on_enter_battlefield`, which is only called during trigger resolution.
- **Graveyard search includes Woodland Sleuth itself when applicable**: Logically PASS (the filter uses `registry.card_data(o.card_id).map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))` which would correctly include the Sleuth's own card entry), but this path is never reached due to the bugs above.
- **Graveyard zone filtered by owner, not controller** (MTG rule: "your graveyard" means cards you own): PASS — `state.objects_in_zone(Zone::Graveyard, controller)` uses `obj.owner == player` for the graveyard zone (state.rs lines 600–607).
- **Morbid flag reset at turn boundary**: PASS — `state.creature_died_this_turn = false` is set in engine.rs line 2888 when advancing to a new turn.
- **Card data matches oracle** (mana cost {3}{G}, 2/3, Creature — Human Scout, no flashback, no continuous effects): PASS — all fields in `card_data()` match the Scryfall oracle data exactly.

### Test coverage

- Morbid active — returns a random creature card from graveyard to hand: `mtg-engine/tests/tier11_cards.rs:378` (`woodland_sleuth_morbid_returns_creature`) — TESTED
- No morbid — no return: `mtg-engine/tests/tier11_cards.rs:396` (`woodland_sleuth_no_morbid_no_return`) — TESTED
- Intervening-if: trigger must NOT go on the stack when no creature died this turn: NOT TESTED
- Woodland Sleuth dies in response to its own morbid trigger — ability still resolves and may return Sleuth to hand: NOT TESTED
- Random selection is deferred until resolution (not at trigger collection): NOT TESTED
- Graveyard with only non-creature cards — no cards returned: NOT TESTED
- Empty graveyard when trigger resolves — no cards returned: NOT TESTED
