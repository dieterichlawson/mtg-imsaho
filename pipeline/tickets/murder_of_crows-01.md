---
id: murder_of_crows-01
status: closed-duplicate
card: Murder of Crows
card_file: mtg-engine/src/cards/isd/murder_of_crows.rs
created: 2026-04-15T03:45:47Z
audit_run_id: 2026-04-14-murder_of_crows-audit
audit_model: opus
audit_tokens: 17841
audit_duration: 401
duplicate_of: merged-trigger-source-zone-gate-02
---

## Audit Finding

**Oracle text:**
> Whenever another creature dies, you may draw a card. If you do, discard a card.

**Code:**
> `murder_of_crows.rs:41-44`:
> ```rust
> let controller = match state.get_object(self_id) {
>     Some(o) if o.zone == Zone::Battlefield => o.controller,
>     _ => return,
> };
> ```

**Description:**
The `on_any_creature_dies` handler gates on `self_id` being on the battlefield (zone == Zone::Battlefield). When Murder of Crows dies simultaneously with another creature (e.g., board wipe, mutual combat trade), the trigger dispatch correctly creates a DeathWatch trigger via the `simultaneously_dead` list (triggers.rs:613-644), but when that trigger resolves, the handler finds Murder of Crows in the graveyard and returns without executing the draw/discard effect. Per CR 603.6c and 603.10, a triggered ability that has been put on the stack resolves even if the source has left the battlefield. Since this ability does not reference the source permanent (it just draws and discards for the controller), it should resolve fully. This directly contradicts the Scryfall ruling. The correct pattern is used by Falkenrath Noble, which reads the controller without gating on zone. This is the same Bug BT pattern documented in audit_trigger_dispatch_family.rs:72-78.

**Engine path:**
- mtg-engine/src/cards/isd/murder_of_crows.rs:41-44

**Required check:** 8b

**Affected cards:**
- Murder of Crows
- Abattoir Ghoul (same zone-gate pattern, documented in Bug BT)
- Rage Thrower (same pattern, per audit_trigger_dispatch_family.rs:74)
- Selhoff Occultist (same pattern, per audit_trigger_dispatch_family.rs:75)

## Tests

### murder_of_crows_simultaneous_death_trigger_resolves
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Murder of Crows on the battlefield controlled by P0 with cards in library. Move Murder of Crows to the graveyard (simulating simultaneous death). Call `on_any_creature_dies` with a dummy dead creature. Assert that `state.awaiting_action` is `Some(YesNo { .. })` — the draw choice should be presented even though Murder of Crows is in the graveyard.
