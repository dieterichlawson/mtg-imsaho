---
id: angel_of_flight_alabaster-03
status: new
card: Angel of Flight Alabaster
card_file: mtg-engine/src/cards/isd/angel_of_flight_alabaster.rs
created: 2026-04-14T21:22:40Z
audit_run_id: 2026-04-14-angel_of_flight_alabaster-audit
audit_model: opus
audit_tokens: 20797
audit_duration: 406
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.

**Code:**
> triggers.rs:1306-1311 — `resolve_next_trigger` dispatches the UpkeepTrigger directly to `behavior.on_upkeep()` without re-checking target legality.
> engine.rs:3606-3609 — `ReturnToHand` handler calls `state.move_object(*id, Zone::Hand, registry)` without verifying the object is still in the graveyard.

**Description:**
Per CR 608.2b, a triggered ability with targets must verify that all targets are still legal when it resolves. If the targeted Spirit card is moved out of the graveyard between targeting and resolution (e.g., exiled by Tormod's Crypt, returned to hand by another effect), the trigger should be removed from the stack with no effect. Instead, the engine resolves the trigger unconditionally: `on_upkeep` passes the stale target to `apply_pending_effect(ReturnToHand)`, which calls `move_object(id, Zone::Hand)` on whatever zone the object is currently in. This could move a card from exile to hand, or attempt to "return" a card that's already in hand. This is a known engine-wide issue (documented in auditor-insights.md) that affects all targeted triggered abilities.

**Engine path:**
- triggers.rs:1306-1311 (no target re-check before dispatch)
- engine.rs:3606-3609 (ReturnToHand has no zone guard)

**Required check:** 8b (also documented in auditor-insights.md)

**Affected cards:**
- Angel of Flight Alabaster
- All cards with targeted triggered abilities (Snapcaster Mage, etc.)

