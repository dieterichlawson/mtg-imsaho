---
id: into_the_maw_of_hell-01
status: fixed
card: Into the Maw of Hell
audit_run_id: 2026-04-19-into_the_maw_of_hell-audit
audit_model: sonnet
audit_tokens: 27027
audit_duration: 478
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: same HasCardType fix; TwoTargets land sub-requirement now enumerates non-token lands; regression coverage present
---

## Audit Finding

**Oracle text:**
> Destroy target land. Into the Maw of Hell deals 13 damage to target creature.

**Code:**
> TargetFilter::HasCardType(types) => {
    types.iter().any(|t| obj.card_types.contains(t))
}

**Description:**
The card declares `target_requirement() = TwoTargets(PermanentWithFilter(HasCardType([Land])), Creature)`. When the engine generates cast targets, the `TwoTargets` branch in `generate_cast_actions_with_targets` (engine.rs:1689) calls `valid_targets_for_req` for the land sub-requirement `PermanentWithFilter(HasCardType([Land]))`. That function (engine.rs:1777–1783) applies `matches_target_filter(o, HasCardType([Land]), registry)` as a mandatory pre-filter. The `HasCardType` branch in `matches_target_filter` (engine.rs:2103–2104) checks only `obj.card_types.contains(&CardType::Land)` with no registry fallback. Non-token permanents have `card_types: Vec::new()` on their battlefield object, so every non-token land returns false. The land-target list is always empty, the Cartesian product is empty, and `Into the Maw of Hell` can never be cast targeting a non-token land — even when one is on the battlefield. The card's own `is_valid_target` correctly uses the registry (`registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Land))`), but it is never reached because `matches_target_filter` filters all lands out first.

**Engine path:** mtg-engine/src/engine.rs:2103

**Required check:** 8f

## Tests

### cast_targeting_basic_land
Scenario: Into the Maw of Hell should appear as a legal cast action when a basic Forest (non-token land) and a creature are on the battlefield; verify the spell is offered and resolves, destroying the land and dealing 13 damage to the creature.

### partial_resolution_land_destroyed_in_response
Scenario: Cast Into the Maw of Hell targeting a land and a creature; opponent destroys the land in response; the spell should still resolve and deal 13 damage to the creature (ruling 2).

