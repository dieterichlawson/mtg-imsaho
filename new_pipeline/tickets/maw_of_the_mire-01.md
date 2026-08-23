---
id: maw_of_the_mire-01
status: new
card: Maw of the Mire
audit_run_id: 2026-04-19-maw_of_the_mire-audit
audit_model: sonnet
audit_tokens: 18769
audit_duration: 436
---

## Audit Finding

**Oracle text:**
> Destroy target land.

**Code:**
> fn target_requirement(&self) -> TargetRequirement {
    TargetRequirement::PermanentWithFilter(
        TargetFilter::HasCardType(vec![CardType::Land]),
    )
}

// engine.rs:2103-2104 — matches_target_filter HasCardType branch:
TargetFilter::HasCardType(types) => {
    types.iter().any(|t| obj.card_types.contains(t))
}

**Description:**
The `build_cast_target_spec` function (called for AI/LLM players to enumerate valid targets) routes `TargetRequirement::PermanentWithFilter` through `valid_targets_for_req` (engine.rs:1931), which pre-filters candidates through `matches_target_filter`. The `HasCardType` branch in `matches_target_filter` (engine.rs:2103-2104) checks only `obj.card_types` with no registry fallback. Because `create_object` initialises `card_types: Vec::new()` for every non-token permanent, all non-token lands on the battlefield have empty `obj.card_types` and are excluded by this filter. AI/LLM players therefore see zero valid targets for Maw of the Mire even when the opponent controls multiple non-token lands, making the card appear uncastable. Human and random players are unaffected: the action-generation path at engine.rs:1655-1669 contains a dedicated `PermanentWithFilter` branch that explicitly skips `matches_target_filter` (comment: 'Actual filtering is done by the card\'s is_valid_target') and delegates entirely to `is_valid_target`, which uses the registry correctly.

**Engine path:** mtg-engine/src/engine.rs:2103

**Required check:** 8f

**Affected cards:**
- Ghost Quarter

## Tests

### maw_ai_land_target_visible
Scenario: AI player controls Maw of the Mire and sufficient mana; opponent controls a non-token Forest. The `build_cast_target_spec` call for Maw should return a `SingleTarget` list that includes the Forest, but currently returns an empty list, causing the AI to never cast the spell.

