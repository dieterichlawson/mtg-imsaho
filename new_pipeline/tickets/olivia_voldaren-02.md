---
id: olivia_voldaren-02
status: new
card: Olivia Voldaren
audit_run_id: 2026-04-19-olivia_voldaren-audit
audit_model: sonnet
audit_tokens: 22278
audit_duration: 724
---

## Audit Finding

**Oracle text:**
> Gain control of target Vampire for as long as you control Olivia Voldaren.

**Code:**
> fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
    // When Olivia leaves the battlefield, return all stolen creatures to their original controllers.
    ...
}

**Description:**
The control-steal effect from ability 1 is reverted only inside on_leave_battlefield, which fires when Olivia changes zones. Per CR 611.2b, a "for as long as" duration ends the moment its condition becomes false — regardless of how. If an opponent temporarily gains control of Olivia via a non-zone-change effect (e.g., Act of Treason), you no longer control Olivia while she remains on the battlefield. The "for as long as" condition is then false and the stolen Vampires must immediately revert to their original controllers. Nothing in the implementation detects a controller change on Olivia that is not accompanied by a zone change: there is no upkeep check, no controller-change event, and no continuous re-evaluation hook. The engine has no general controller-change notification mechanism, so this requires a card-level workaround such as an end-step trigger that compares the current controller of object_id against the stored activating controller and reverts stolen creatures if they differ.

**Engine path:** mtg-engine/src/cards/isd/olivia_voldaren.rs:155

**Required check:** 8h

## Tests

### stolen_vampires_returned_when_olivia_control_changes_without_zone_change
Scenario: Player A steals player B's Vampire with Olivia's ability; player B uses Act of Treason to take control of Olivia; verify the Vampire immediately returns to player B's control even though Olivia is still on the battlefield.

### stolen_vampires_returned_when_olivia_leaves_battlefield
Scenario: Player A steals a Vampire with Olivia's ability; Olivia is then destroyed; verify the Vampire returns to its original controller. (Regression guard for the existing on_leave_battlefield path.)

