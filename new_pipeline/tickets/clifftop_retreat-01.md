---
id: clifftop_retreat-01
status: new
card: Clifftop Retreat
audit_run_id: 2026-04-19-clifftop_retreat-audit
audit_model: sonnet
audit_tokens: 9206
audit_duration: 182
---

## Audit Finding

**Oracle text:**
> This land enters tapped unless you control a Mountain or a Plains.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::EntersBattlefield,
        description: "enters tapped unless you control a Mountain or a Plains".into(),
        target_requirement: None,
    },
],

**Description:**
"This land enters tapped unless you control a Mountain or a Plains" is a CR 614.1d replacement effect — it modifies the entering event itself and does not use the stack. The implementation models it as a triggered ability (TriggerKind::EntersBattlefield) that taps the land in on_enter_battlefield when the trigger resolves. This has two concrete observable bugs. First, there is a priority window between the land entering (untapped) and the trigger resolving: the controller can tap Clifftop Retreat for {R} or {W} in response to its own ETB trigger, even when the land should have entered tapped. Second, the matching-land check (controller_has_matching_land) runs at trigger-resolution time rather than at the moment of entering: an opponent can respond to the trigger by bouncing or destroying the controller's only Mountain or Plains, causing the land to be tapped even though the condition was satisfied when the land entered. With a correct replacement effect there is no stack entry and no response window — the tapping is decided before EnteredBattlefield is even emitted. Additionally, even when the condition IS satisfied and the land should enter untapped, the trigger still goes on the stack unnecessarily, giving players a spurious priority window. The same pattern affects the other four Innistrad check lands, each of which has an existing ticket.

**Engine path:** mtg-engine/src/cards/isd/clifftop_retreat.rs:47

**Required check:** 6

**Affected cards:**
- Woodland Cemetery
- Hinterland Harbor
- Isolated Chapel
- Sulfur Falls

## Tests

### clifftop_retreat_tap_for_mana_in_priority_window
Scenario: Play Clifftop Retreat when controller controls no Mountain or Plains; controller taps it for {R} in response to the ETB trigger before the trigger resolves — the land should have entered tapped via replacement effect, making this mana activation impossible

### clifftop_retreat_condition_evaluated_at_entry_not_resolution
Scenario: Play Clifftop Retreat while controlling a Plains; opponent destroys or bounces the Plains in response to the ETB trigger; trigger resolves and incorrectly taps the land — with a replacement effect the condition would have been evaluated at entry time when the Plains was present, and the land would remain untapped

