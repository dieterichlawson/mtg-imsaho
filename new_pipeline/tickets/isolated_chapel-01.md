---
id: isolated_chapel-01
status: new
card: Isolated Chapel
audit_run_id: 2026-04-19-isolated_chapel-audit
audit_model: sonnet
audit_tokens: 10757
audit_duration: 207
---

## Audit Finding

**Oracle text:**
> This land enters tapped unless you control a Plains or a Swamp.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::EntersBattlefield,
        description: "enters tapped unless you control a Plains or a Swamp".into(),
    target_requirement: None,
    },
],

**Description:**
"This land enters tapped unless you control a Plains or a Swamp" is a static replacement effect under CR 614.1d — it modifies the entering event before it occurs and produces no stack entry. The implementation instead uses a TriggerKind::EntersBattlefield triggered ability whose on_enter_battlefield handler taps the land at trigger-resolution time. This creates two observable bugs: (1) The land enters untapped and a trigger is placed on the stack, giving players a priority window during which the land is visibly untapped and can legally be tapped for {W} or {B} — providing free mana in cases where the land should have entered tapped. (2) The condition check ("you control a Plains or Swamp") is evaluated when the trigger resolves, not at the moment of entry; if an opponent destroys the qualifying land in response to the trigger, the Chapel is incorrectly tapped even though the condition was satisfied at entry time. The correct fix is a pre-entry hook (e.g., entering_tapped(state, id, registry) -> bool) called before EnteredBattlefield is emitted in move_object, as documented in auditor-insights.md for this card cycle.

**Engine path:** mtg-engine/src/cards/isd/isolated_chapel.rs:47

**Required check:** 8b

**Affected cards:**
- Woodland Cemetery
- Clifftop Retreat
- Hinterland Harbor
- Sulfur Falls

## Tests

### chapel_enters_tapped_no_mana_window
Scenario: Isolated Chapel enters while the controller has no Plains or Swamp; verify the land is tapped from the moment it enters with no priority window during which the untapped land's mana abilities are available.

### chapel_condition_checked_at_entry_time
Scenario: Controller has a Plains when Isolated Chapel enters; opponent destroys the Plains before the triggered ability resolves; land should remain untapped (condition was true at entry time), but the bug causes it to be tapped.

