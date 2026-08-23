---
id: evil_twin-01
status: new
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
---

## Audit Finding

**Oracle text:**
> You may have this creature enter as a copy of any creature on the battlefield

**Code:**
> pub fn creature_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != exclude)
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

**Description:**
The copy choice for Evil Twin's ETB is built using `creature_targets_except`, which calls `can_be_targeted_by`. This filters out hexproof creatures (if controlled by an opponent) and creatures with protection from Evil Twin's color or type. However, the Oracle text says 'any creature on the battlefield' — the copy is a replacement-effect choice (CR 614.12b), not targeting. The word 'target' does not appear. Hexproof (702.11) and protection (702.16) only restrict targeting actions (CR 115.1 defines targeting as requiring the word 'target'). A hexproof creature controlled by the opponent cannot be targeted but CAN be chosen for copying. The engine incorrectly excludes such creatures from the copy-choice candidate list.

**Engine path:** mtg-engine/src/cards/helpers.rs:154

**Required check:** 8f

## Tests

### evil_twin_copy_choice_includes_hexproof_creature
Scenario: Opponent controls a hexproof creature; Evil Twin enters and the copy-choice candidate list should include the hexproof creature but currently excludes it.

