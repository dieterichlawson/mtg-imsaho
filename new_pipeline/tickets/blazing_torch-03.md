---
id: blazing_torch-03
status: new
card: Blazing Torch
audit_run_id: 2026-04-19-blazing_torch-audit
audit_model: sonnet
audit_tokens: 23515
audit_duration: 1971
---

## Audit Finding

**Oracle text:**
> [2009-10-01] If a Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player. Only the creature's controller may activate the ability — but since that player can't sacrifice Blazing Torch (a permanent they don't control), the ability's cost can't be paid.

**Code:**
> for attached in state.objects.values() {
    if attached.zone == Zone::Battlefield && attached.attached_to == Some(obj_id) {
        if let Some(behavior) = registry.get(attached.card_id) {
            for ab in behavior.activated_abilities(state, obj_id, registry) {
                abilities.push((attached.card_id, ab));
            }
        }
    }
}

**Description:**
The legal-action enumeration loop (engine.rs:682-688) collects activated abilities from all attached permanents without checking whether the attached permanent's controller matches the creature's controller. If Blazing Torch is controlled by player A but is somehow attached to a creature controlled by player B, the loop will include the torch's damage ability in player B's legal actions. Player B can legally satisfy the ability's declared cost (tap their creature; the mana cost is free; sacrifice_cost is SacrificeCost::None since the sacrifice is handled manually in on_activate_ability). The on_activate_ability handler (blazing_torch.rs:116-117) then calls crate::destruction::sacrifice(state, torch, registry) unconditionally without verifying that the activating player controls the torch, allowing player B to sacrifice a permanent they don't own. The ruling states that neither player can activate the ability in this scenario because the creature's controller cannot pay the sacrifice cost for a permanent they don't control. The fix requires adding an attached.controller == player guard in the legal-action loop, and/or a controller check in on_activate_ability before sacrificing the found torch.

**Engine path:** mtg-engine/src/engine.rs:682

**Required check:** 8j

## Tests

### cross_controller_ability_not_offered
Scenario: Blazing Torch controlled by player A is attached to a creature controlled by player B (set up via direct state manipulation); neither player's legal actions should include the Blazing Torch damage ability.

