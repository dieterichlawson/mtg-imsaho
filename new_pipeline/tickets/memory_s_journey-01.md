---
id: memory_s_journey-01
status: new
card: Memory's Journey
audit_run_id: 2026-04-19-memory_s_journey-audit
audit_model: sonnet
audit_tokens: 36532
audit_duration: 612
---

## Audit Finding

**Oracle text:**
> Target player shuffles up to three target cards from their graveyard into their library.

**Code:**
> TargetRequirement::TwoTargets(
    Box::new(TargetRequirement::PlayerOnly),
    Box::new(TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCard))),
)

**Description:**
Memory's Journey cannot be cast at all. The `target_requirement` returns `TwoTargets(PlayerOnly, UpToTargets(3, GraveyardCard))`. In `generate_cast_actions_with_targets`, the `TwoTargets` handler (engine.rs:1687) calls `valid_targets_for_req` on both slots; the second slot is `UpToTargets(3, GraveyardCard)`, which hits the catch-all `_ => vec![]` at engine.rs:1893 because `valid_targets_for_req` has no branch for `UpToTargets`. The resulting Cartesian product is always empty, so `cast_actions` is always empty. The guard `if !cast_actions.is_empty()` (engine.rs:1173) then suppresses Memory's Journey from both the AI action list and the human `castable_spells` list. The same failure mode applies to the flashback path. A comment at engine.rs:1710 explicitly references Memory's Journey as the motivation for the 0-count UpToTargets support, confirming the developer was aware of the 'up to N' requirement but did not extend the fix to cover `UpToTargets` nested inside `TwoTargets`. The fix is to add an `UpToTargets(_, inner) => valid_targets_for_req(state, caster, spell_id, inner, behavior, registry)` branch in `valid_targets_for_req`, and to update the `TwoTargets` Cartesian product handler to treat an empty second slot as allowing a single empty selection (so that 'player + 0 cards' remains a valid cast).

**Engine path:** mtg-engine/src/engine.rs:1687

## Tests

### memories_journey_ai_can_cast_player_only
Scenario: An AI player with Memory's Journey in hand, the opponent's graveyard empty, has at least one valid CastSpell action (targeting just the opponent with zero graveyard card targets).

### memories_journey_ai_can_cast_with_graveyard_cards
Scenario: An AI player with Memory's Journey in hand and two cards in the opponent's graveyard has CastSpell actions that include those graveyard cards as targets.

