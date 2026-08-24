---
id: memory_s_journey-02
status: fixed
card: Memory's Journey
audit_run_id: 2026-04-19-memory_s_journey-audit
audit_model: sonnet
audit_tokens: 36532
audit_duration: 612
fixed_sha: e3de0667d9633d33b98ff438075a18b875157cc3
fixed_at: 2026-08-24T00:30:46Z
test_file: mtg-engine/tests/multi_target_and_mill.rs
fix_note: GraveyardCardOwnedByTargetPlayer constrains card targets to the targeted player's graveyard at announcement (CR 601.2c)
---

## Audit Finding

**Oracle text:**
> up to three target cards from their graveyard

**Code:**
> Box::new(TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCard)))

**Description:**
The oracle text 'from their graveyard' constrains card targets to only the targeted player's graveyard. The `GraveyardCard` requirement (engine.rs:1830-1836) enumerates all cards in all graveyards — any player's — without restriction. At announcement (CR 601.2c) a player could declare targets from a third party's graveyard; those targets satisfy `GraveyardCard` zone legality and will not be filtered out. The `on_resolve` handler compensates at resolution by checking `owner == target_player`, silently discarding wrongly-targeted cards, but the rules violation occurs at announcement, not resolution. A correct implementation would use a `GraveyardCardOwnedByTarget` requirement type or implement `is_valid_target` to enforce the ownership constraint. Note: the engine's current `is_valid_target` signature does not receive the co-target list, so enforcing 'from the same player as the player target' requires either a new requirement type or a spec-level cross-target guard.

**Engine path:** mtg-engine/src/cards/isd/memorys_journey.rs:39

## Tests

### memories_journey_cannot_target_wrong_graveyard
Scenario: Player A targets Player B and two cards from Player C's graveyard. Those graveyard cards should not be valid targets at announcement; the engine should offer only cards from Player B's graveyard as valid second-slot targets.

