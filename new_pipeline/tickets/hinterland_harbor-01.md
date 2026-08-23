---
id: hinterland_harbor-01
status: new
card: Hinterland Harbor
audit_run_id: 2026-04-19-hinterland_harbor-audit
audit_model: sonnet
audit_tokens: 7595
audit_duration: 164
---

## Audit Finding

**Oracle text:**
> This land enters tapped unless you control a Forest or an Island.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::EntersBattlefield,
        description: "enters tapped unless you control a Forest or an Island".into(),
        target_requirement: None,
    },
],

**Description:**
"This land enters tapped unless you control a Forest or an Island" is a CR 614.1d replacement effect — it modifies the entering event itself and does not use the stack. The implementation models it as a triggered ability (TriggerKind::EntersBattlefield) whose on_enter_battlefield handler taps the land at trigger-resolution time. This produces two concrete observable bugs. First, there is a priority window between the land entering untapped and the trigger resolving: the controller can tap Hinterland Harbor for {G} or {U} in response to its own ETB trigger, even when the land should have entered tapped and been unavailable to activate. Second, the matching-land check (controller_has_matching_land) runs at trigger-resolution time rather than at entry time: an opponent can respond to the trigger by destroying or bouncing the controller's only Forest or Island, causing the land to be tapped even though the condition was satisfied at the moment the land entered. A correct replacement effect has no stack entry and no response window — the tapping (or not) is decided before EnteredBattlefield is even emitted, exactly as entering_with_counters handles CR 614.1c. This is the same defect filed for the other Innistrad check lands.

**Engine path:** mtg-engine/src/cards/isd/hinterland_harbor.rs:47

**Required check:** 8b

**Affected cards:**
- Woodland Cemetery
- Clifftop Retreat
- Isolated Chapel
- Sulfur Falls

## Tests

### hinterland_harbor_tap_for_mana_in_etb_priority_window
Scenario: Play Hinterland Harbor when controller controls no Forest or Island; controller taps it for {G} in response to its own ETB trigger before the trigger resolves — land should have entered tapped via replacement effect, making this mana activation impossible

### hinterland_harbor_condition_evaluated_at_entry_not_resolution
Scenario: Play Hinterland Harbor while controlling an Island; opponent destroys or bounces the Island in response to the ETB trigger; trigger resolves and incorrectly taps the land — with a replacement effect the condition would have been evaluated at entry time when the Island was present, and the land would remain untapped

