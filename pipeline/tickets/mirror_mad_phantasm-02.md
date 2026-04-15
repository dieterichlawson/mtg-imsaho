---
id: mirror_mad_phantasm-02
status: closed-duplicate
card: Mirror-Mad Phantasm
card_file: mtg-engine/src/cards/isd/mirror_mad_phantasm.rs
created: 2026-04-15T03:36:03Z
audit_run_id: 2026-04-14-mirror_mad_phantasm-audit
audit_model: opus
audit_tokens: 26524
audit_duration: 621
duplicate_of: merged-controller-after-move-01
---

## Audit Finding

**Oracle text:**
> The player puts that card onto the battlefield

**Code:**
> mirror_mad_phantasm.rs:105-108: `state.move_object(phantasm_id, Zone::Battlefield, registry); if let Some(obj) = state.get_object_mut(phantasm_id) { obj.controller = owner; }`

**Description:**
When the found Mirror-Mad Phantasm is put onto the battlefield, `move_object` is called first, which emits an `EnteredBattlefield` event (state.rs:618) using the object's current `controller` field. The explicit `obj.controller = owner` assignment happens AFTER the event is emitted. If the Phantasm was previously controlled by a non-owner (e.g., via Control Magic) before being shuffled into the library, its `controller` field retains the previous controller's value because `move_object`'s zone-change cleanup (state.rs:572-583) does not clear `controller`. The `EnteredBattlefield` event thus carries the stale controller. Any ETB triggers that check the entering creature's controller (e.g., "Whenever a creature enters the battlefield under your control") will fire for the wrong player or fail to fire for the correct player. Per the oracle text, "The player puts that card onto the battlefield" where "the player" is the owner — so the entering Phantasm should be controlled by the owner, and ETB events should reflect that. This is an instance of the known engine pattern documented in auditor-insights.md ("Controller update after move_object causes stale EnteredBattlefield events").

**Engine path:**
- state.rs:617-621 (ETB event emitted with current controller before explicit reassignment)
- state.rs:572-583 (zone-change cleanup does not clear `controller`)
- mirror_mad_phantasm.rs:105-108 (controller set after move_object)

**Required check:** 8a

**Affected cards:**
- Mirror-Mad Phantasm
- All cards that set controller after `move_object` to battlefield (see auditor-insights.md)

## Tests

### etb_event_controller_after_control_magic
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player B controls Mirror-Mad Phantasm (owned by Player A) via a Control Magic-like effect. Player A has a "Whenever a creature enters the battlefield under your control, gain 1 life" trigger. Player B activates Mirror-Mad Phantasm's ability. The Phantasm is shuffled into Player A's library and found during the reveal. When it enters the battlefield, verify that: (1) the Phantasm is controlled by Player A (the owner), and (2) Player A's ETB trigger fires (not Player B's). Currently, the `EnteredBattlefield` event carries Player B as the controller, causing Player A's trigger to miss.
