## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Sturmgeist's power and toughness are each equal to the number of cards in your hand.
Whenever this creature deals combat damage to a player, draw a card.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues

- Draw skipped when Sturmgeist leaves battlefield before trigger resolves (`mtg-engine/src/cards/isd/sturmgeist.rs:46-49`)
  - Oracle text says: `"Whenever this creature deals combat damage to a player, draw a card."`
  - Code does: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` — if Sturmgeist is not in `Zone::Battlefield` when the trigger resolves (e.g., killed by instant-speed removal between trigger collection and resolution), the function returns early without drawing. The oracle text imposes no condition on the source creature being on the battlefield at resolution time; "draw a card" is an unconditional effect once the trigger fires.

### Tricky interactions checked

- **draw-a-card resolves when Sturmgeist is off-battlefield at resolution**: FAIL — the `zone == Zone::Battlefield` guard in `on_combat_damage_to_player` prevents the draw if Sturmgeist has left the battlefield (e.g., killed by instant removal after the trigger is on the stack). Per MTG rules, a triggered ability that says "draw a card" resolves even if the source is no longer on the battlefield, because the draw does not reference the source.
- **dynamic_pt works in all zones (Scryfall ruling 2011-09-22)**: PASS — `dynamic_pt` does not check the zone of the Sturmgeist object; it calls `state.get_object(object_id)?.controller` (which persists through zone changes) then counts `Zone::Hand` cards for that controller. `effective_power`/`effective_toughness` in `state.rs` invoke `dynamic_pt` without any zone restriction on the queried object.
- **dynamic_pt re-evaluates continuously**: PASS — `dynamic_pt` is called at every P/T query; it is not snapshotted at ETB or at any other fixed point, so the P/T tracks the hand size correctly as it changes.
- **Combat damage trigger fires correctly (dispatch in triggers.rs)**: PASS — `triggers.rs:489-511` handles `GameEvent::CombatDamageDealt` with `DamageTarget::Player`, checks `obj.zone == Zone::Battlefield && obj.power.is_some()`, then looks up `trigger_description` for `TriggerKind::CombatDamageToPlayer`; Sturmgeist's `triggered_abilities` vec contains exactly that kind with a non-empty description (`"draw a card"`), so the trigger is collected and pushed onto the stack.
- **Trigger is mandatory (no "you may")**: PASS — oracle text says "draw a card" with no "you may"; the code unconditionally calls `draw_cards` (when it reaches that point). No optional choice is presented.
- **Hand count uses correct player (controller's hand, not owner's)**: PASS — `dynamic_pt` reads `state.get_object(object_id)?.controller` and passes that to `objects_in_zone(Zone::Hand, controller)`. Hand objects filter by `owner == player`, which correctly reflects the controller's own cards in hand under normal game conditions.
- **Trigger dispatch zone check vs. resolution zone check**: PASS for dispatch (creature must be on battlefield when damage is dealt, which it always will be), FAIL for resolution (the handler unnecessarily blocks draw if creature left before resolution — see above).

### Test coverage

- **P/T equals hand size**: `mtg-engine/tests/tier6_cards.rs:286` (`sturmgeist_pt_equals_hand_size`) — TESTED
- **dynamic_pt in all zones**: NOT TESTED — no test checks `effective_power`/`effective_toughness` on a Sturmgeist object in the graveyard, library, or exile
- **Draw-a-card trigger fires after combat damage**: NOT TESTED — there is no test that fires the `CombatDamageToPlayer` event for Sturmgeist and checks that a card is drawn
- **Draw resolves when Sturmgeist off-battlefield**: NOT TESTED
- **Trigger dispatch fires for correct TriggerKind**: NOT TESTED directly (covered implicitly only if draw test existed)
