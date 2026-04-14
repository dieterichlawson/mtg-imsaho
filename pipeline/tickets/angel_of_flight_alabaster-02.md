---
id: angel_of_flight_alabaster-02
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
> triggers.rs:1307 — `if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)` gates the entire resolution on the source permanent still being on the battlefield.
> angel_of_flight_alabaster.rs:54-57 — `match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return }` performs the same gate at the card level.

**Description:**
If Angel of Flight Alabaster is removed from the battlefield after its upkeep trigger goes on the stack but before it resolves (e.g., destroyed in response), the trigger silently does nothing. Per CR 603.7b, a triggered ability that doesn't reference the source object should still resolve — the Angel's ability says "return target Spirit card from your graveyard to your hand," which doesn't need any information from the source permanent. The "your" was determined when the trigger was put on the stack (it refers to the controller of the ability). The trigger should resolve and return the targeted Spirit regardless of whether the Angel is still on the battlefield. The engine-level zone gate (triggers.rs:1307) prevents `on_upkeep` from even being called, and the card-level gate (line 54-57) would independently prevent resolution. Both gates are incorrect for this ability.

**Engine path:**
- triggers.rs:1306-1311 (engine zone gate)
- angel_of_flight_alabaster.rs:53-67 (card zone gate)

**Required check:** 8b

**Affected cards:**
- Angel of Flight Alabaster
- All cards with upkeep/end-step/end-combat triggered abilities that don't reference their source: the engine applies the same zone gate pattern across all step-trigger variants (triggers.rs:1299-1318)

