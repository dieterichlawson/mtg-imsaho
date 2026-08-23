---
id: sulfur_falls-01
status: fixed
card: Sulfur Falls
audit_run_id: 2026-04-19-sulfur_falls-audit
audit_model: sonnet
audit_tokens: 10135
audit_duration: 234
fixed_sha: 5c9fc98eed8d4f0b9ed73646882b288f0f55545d
fixed_at: 2026-08-23T20:10:52Z
test_file: mtg-engine/tests/enters_tapped_replacement.rs
fix_note: cluster fix: CardBehavior::enters_tapped applies the tap as a CR 614.1d replacement effect, not an ETB trigger
---

## Audit Finding

**Oracle text:**
> This land enters tapped unless you control an Island or a Mountain.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::EntersBattlefield,
        description: "enters tapped unless you control an Island or a Mountain".into(),
        target_requirement: None,
    },
],

**Description:**
Per CR 614.1c, 'enters the battlefield tapped' wording is a static replacement effect that modifies the zone-change event before it occurs — no stack entry is created and no priority is granted. The implementation instead uses TriggerKind::EntersBattlefield with a has_etb_handler()/on_enter_battlefield handler that evaluates the condition and taps the land at trigger-resolution time. This produces two concrete bugs. First, a spurious stack entry is created on every Sulfur Falls ETB — even when the controller has a qualifying Island or Mountain and the trigger would do nothing — granting players an incorrect priority window while the land sits untapped on the battlefield. Second, the condition 'you control an Island or a Mountain' is evaluated at trigger-resolution time, not at entry time: if an opponent responds to the ETB trigger by destroying the controller's only qualifying land, controller_has_matching_land() now returns false and the land is incorrectly tapped, even though the replacement effect should have been locked in at entry (land enters untapped). The state.rs move_object() function already has a pre-entry hook pattern (entering_with_counters at line 754, called before EnteredBattlefield is emitted at line 656) that would support the correct fix: a new CardBehavior::entering_tapped(state, id, registry) -> bool hook called before the EnteredBattlefield event is emitted.

**Engine path:** mtg-engine/src/cards/isd/sulfur_falls.rs:47

**Affected cards:**
- Woodland Cemetery
- Clifftop Retreat
- Hinterland Harbor
- Isolated Chapel

## Tests

### sulfur_falls_spurious_stack_entry_when_condition_met
Scenario: Controller has an Island on the battlefield; Sulfur Falls enters the battlefield; verify no trigger appears on the stack and the land enters untapped without a priority window.

### sulfur_falls_condition_locked_at_entry_not_resolution
Scenario: Controller has an Island when Sulfur Falls enters; opponent destroys the Island in response to the ETB trigger; verify Sulfur Falls remains untapped because the replacement effect should have been applied at entry time when the condition was satisfied.

