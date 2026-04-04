## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
**Type line**: Creature — Elemental
**Status**: ISSUE

### Code issues

- Upkeep trigger does not resolve if Splinterfright has left the battlefield between trigger collection and resolution
  - Oracle text says: `"At the beginning of your upkeep, mill two cards."`
  - Code does: In `mtg-engine/src/triggers.rs:954-959`, `resolve_next_trigger` checks `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before calling `on_upkeep`. If Splinterfright has been destroyed or exiled after its trigger was put on the stack, the zone check fails and `on_upkeep` is never called — the mill is skipped entirely. Additionally, `mtg-engine/src/cards/isd/splinterfright.rs:51-54` contains a redundant guard: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };`. Per MTG rules (CR 603.3–603.4), a triggered ability on the stack resolves independently of its source's location; "At the beginning of your upkeep, mill two cards" has no source-persistence condition, so the mill must happen even if Splinterfright has been destroyed in response.

### Tricky interactions checked

- "Your upkeep" guard (trigger fires only on controller's upkeep): The trigger is collected for ALL battlefield permanents every upkeep; `on_upkeep` correctly guards with `if state.active_player != controller { return; }` at `splinterfright.rs:56-58`. PASS
- Splinterfright counts itself when in graveyard (ruling: "If Splinterfright is in your graveyard, it will count itself."): `dynamic_pt` does not zone-check the source object; it calls `state.objects_in_zone(Zone::Graveyard, controller)` which returns all creature cards in the GY including Splinterfright itself (has `power: Some(0)`), so it correctly self-counts. PASS
- Milling with fewer than 2 cards in library (ruling: "If Splinterfright's controller has only one card in their library when its triggered ability resolves, they put that card into their graveyard."): `mill_cards` in `engine.rs:2755-2771` loops and `break`s when `library_order.is_empty()`, so it mills whatever is available without penalty. PASS
- Dynamic P/T continuously re-evaluated (not snapshot at ETB): `effective_power` calls `dynamic_pt` live each time it is invoked; P/T updates whenever the graveyard count changes. PASS
- P/T works in all zones (ruling: "works in all zones, not just the battlefield"): `dynamic_pt` is zone-agnostic — it gets the object's controller regardless of zone, then counts GY creatures. `effective_power`/`effective_toughness` in `state.rs` work for any zone. The battlefield and view layers both compute P/T correctly while Splinterfright is on the battlefield. PASS (with note that `card_view` in `view.rs` uses raw `obj.power = Some(0)` for graveyard display rather than calling `effective_power`, so the graveyard UI shows 0/0 — a cosmetic UI gap but not a game-mechanic bug)
- Trample keyword active: `card_data()` declares `keywords: vec![Keyword::Trample]`; `combat.rs:198-260` checks `has_keyword(attacker_id, Keyword::Trample, registry)` and routes excess damage to the defending player. PASS
- Creature-card detection via `power.is_some()`: The filter `o.power.is_some()` is the engine's standard proxy for "is a creature card" (non-creature cards have `power: None`). Consistent with `boneyard_wurm.rs` which uses the same approach. PASS
- `objects_in_zone(Zone::Graveyard, controller)` filters by `obj.owner` (per `state.rs:601-607`), so it counts the controller's owned cards — correct per MTG's definition of "your graveyard." PASS
- Trigger fires correctly during opponent's upkeep when they control Splinterfright: Trigger is collected; card code returns early because `state.active_player != controller`. Functionally correct (no-op on stack). PASS

### Test coverage

- Splinterfright mills 2 on upkeep: `tier7_cards.rs:43` (`splinterfright_mills_on_upkeep`) — TESTED
- Dynamic P/T equals creature cards in graveyard (while on battlefield): NOT TESTED (Boneyard Wurm has a P/T test at `tier7_cards.rs:18`, but no equivalent for Splinterfright)
- Splinterfright counting itself when in the graveyard: NOT TESTED
- Trigger fires only on controller's upkeep, not opponent's: NOT TESTED
- Mill with fewer than 2 cards in library: NOT TESTED
- Mill trigger resolves if Splinterfright leaves the battlefield before resolution: NOT TESTED (this is the failing scenario; no test catches the bug)
