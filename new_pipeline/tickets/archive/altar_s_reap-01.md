---
id: altar_s_reap-01
status: could_not_confirm
card: Altar's Reap
audit_run_id: 2026-04-18-altar_s_reap-audit
audit_model: sonnet
audit_tokens: 10071
audit_duration: 184
---

## Audit Finding

**Oracle text:**
> As an additional cost to cast this spell, sacrifice a creature.

**Code:**
> // Backward compatibility: if sacrifice is None but the spell has
// AdditionalCost::SacrificeCreature, auto-sacrifice the first creature.
use crate::cards::AdditionalCost;
let needs_sac = registry.get(card_id)
    .is_some_and(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::SacrificeCreature)));
if needs_sac {
    let creature = new_state.objects_in_zone(Zone::Battlefield, player)
        .iter()
        .find(|o| o.power.is_some())
        .map(|o| o.id);
    if let Some(cid) = creature {
        ...
        crate::destruction::sacrifice(&mut new_state, cid, registry);
    }
}

**Description:**
When `Action::CastSpell { sacrifice: None, ... }` is submitted for Altar's Reap (or any spell with `AdditionalCost::SacrificeCreature`), the engine silently sacrifices the first creature found via iterator order rather than requiring the player to specify a choice. Per CR 601.2h, costs are paid by the player in whatever order they choose — when a cost requires selecting a permanent ("sacrifice a creature"), the player must actively pick which permanent to pay with. The `legal_actions()` function correctly expands the spell into one `Action::CastSpell` per eligible creature, each carrying `sacrifice: Some(id)`, so real gameplay produces distinct actions. But `submit_action` accepts `sacrifice: None` without panicking or rejecting it, falling through to the backward-compat auto-select. The consequence is that the standard `cast_and_resolve` test helper (which always passes `sacrifice: None`) never tests the player-choice path, and any caller that submits a raw `CastSpell` with `sacrifice: None` on a multi-creature board will silently sacrifice a creature the player did not choose.

**Engine path:** mtg-engine/src/engine.rs:2429

**Required check:** 8i

**Affected cards:**
- Infernal Plunge

## Tests

### altars_reap_sacrifice_choice_respected_multi_creature
Scenario: With two creatures on the battlefield, cast Altar's Reap specifying the second creature as the sacrifice; verify that creature (not the first) ends up in the graveyard and the first creature is unharmed.

### altars_reap_legal_actions_expands_one_action_per_eligible_creature
Scenario: With three creatures on the battlefield, verify that `legal_actions` returns three distinct `Action::CastSpell` variants for Altar's Reap, each carrying a different `sacrifice: Some(id)`.

## Test Run Results

- **altars_reap_sacrifice_choice_respected_multi_creature** — rejected
  - explanation: The engine already handles sacrifice: Some(id) correctly at engine.rs:2423 — the if-let branch sacrifices exactly the specified creature and the else (auto-pick) branch is skipped entirely. A test that casts Altar's Reap with sacrifice: Some(creature2) and asserts creature2 ends up in the graveyard while creature1 stays on the battlefield passes against the current code, confirming no bug here. The backward-compat auto-pick is only reachable when sacrifice: None is supplied, which is the actual defect described in the ticket (accepting None silently), not the choice-respected path.
- **altars_reap_legal_actions_expands_one_action_per_eligible_creature** — rejected
  - explanation: legal_actions already correctly expands Altar's Reap into one CastSpell action per eligible sacrifice creature at engine.rs:1110-1124. With three creatures on the battlefield a test filtering legal_actions for CastSpell variants with non-None sacrifice IDs finds exactly three distinct entries (one per creature), confirming this code path is correct. The test passes against the current code.

