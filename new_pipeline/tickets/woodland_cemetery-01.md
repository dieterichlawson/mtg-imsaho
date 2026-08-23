---
id: woodland_cemetery-01
status: fixed
card: Woodland Cemetery
audit_run_id: 2026-04-19-woodland_cemetery-audit
audit_model: sonnet
audit_tokens: 18340
audit_duration: 329
fixed_sha: 5c9fc98eed8d4f0b9ed73646882b288f0f55545d
fixed_at: 2026-08-23T20:10:52Z
test_file: mtg-engine/tests/enters_tapped_replacement.rs
fix_note: cluster fix: CardBehavior::enters_tapped applies the tap as a CR 614.1d replacement effect, not an ETB trigger
---

## Audit Finding

**Oracle text:**
> This land enters tapped unless you control a Swamp or a Forest.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::EntersBattlefield,
        description: "enters tapped unless you control a Swamp or a Forest".into(),
        target_requirement: None,
    },
],

**Description:**
"This land enters tapped unless you control a Swamp or a Forest" is a CR 614.1d replacement effect — it modifies the entering event itself, and does not use the stack. The implementation models it as a triggered ability (`TriggerKind::EntersBattlefield`) that calls `on_enter_battlefield` when the trigger resolves. This has two concrete observable bugs. First, there is a priority window between the land entering (untapped) and the trigger resolving: the controller can tap Woodland Cemetery for {B} or {G} in response to its own ETB trigger, even when the land should have entered tapped. Second, the matching-land check (`controller_has_matching_land`) runs at trigger-resolution time rather than at the moment of entering: an opponent can respond to the trigger by bouncing the controller's only Swamp or Forest, causing the land to be tapped even though the condition was satisfied when the land entered. With a correct replacement effect there is no stack entry and no response window — the tapping happens before `EnteredBattlefield` is even emitted. The same pattern affects all five Innistrad check lands: Clifftop Retreat, Hinterland Harbor, Isolated Chapel, and Sulfur Falls use an identical implementation.

**Engine path:** mtg-engine/src/cards/isd/woodland_cemetery.rs:47

**Required check:** 6

**Affected cards:**
- Clifftop Retreat
- Hinterland Harbor
- Isolated Chapel
- Sulfur Falls

## Tests

### check_land_tap_for_mana_in_priority_window
Scenario: Play Woodland Cemetery when controller controls no Swamp or Forest; controller taps it for {B} in response to the ETB trigger before the trigger resolves — land should have entered tapped via replacement effect, making this mana activation impossible

### check_land_condition_evaluated_at_entry_not_resolution
Scenario: Play Woodland Cemetery while controlling a Forest; opponent destroys or bounces the Forest in response to the ETB trigger; trigger resolves and incorrectly taps the land — with a replacement effect the condition would have been evaluated at entry time when the Forest was present, and the land would remain untapped

