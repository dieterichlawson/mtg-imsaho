---
id: daybreak_ranger-01
status: new
card: Daybreak Ranger
audit_run_id: 2026-04-19-daybreak_ranger-audit
audit_model: sonnet
audit_tokens: 29964
audit_duration: 567
---

## Audit Finding

**Oracle text:**
> {T}: This creature deals 2 damage to target creature with flying.
--- Back Face ---
{R}, {T}: This creature fights target creature.

**Code:**
> fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
`generate_ability_targets` (engine.rs:1991) receives `source_id: ObjectId` but the filter step calls `can_be_targeted(state, o.id, controller, registry)` — the no-source wrapper — instead of `can_be_targeted_by(state, o.id, controller, Some(source_id), registry)`. `can_be_targeted_by` only executes the protection-from-source check (engine.rs:1462–1466) when `source_id` is `Some`; with `None` that check is silently skipped. For Daybreak Ranger's front face (a green source), a flying creature with protection from green is an illegal target but will be offered and accepted. For Nightfall Predator's fight ability (a red source), a creature with protection from red is similarly offered. Per CR 702.16b, protection prevents targeting; the omission of the source ID from the targeting filter violates this on every activated ability in the engine.

**Engine path:** mtg-engine/src/engine.rs:1447

**Required check:** 8f

**Affected cards:**
- Daybreak Ranger
- Nightfall Predator
- Kessig Wolf Run
- Stensia Bloodhall

## Tests

### daybreak_ranger_cannot_target_protection_from_green_flying_creature
Scenario: Opponent controls a flying creature enchanted with protection from green. Activate Daybreak Ranger's {T} ability. Assert that the flying creature is NOT in the set of legal targets offered to the player.

### nightfall_predator_cannot_target_protection_from_red_creature
Scenario: Opponent controls a creature with protection from red (e.g., via Apostle's Blessing). Transform Daybreak Ranger into Nightfall Predator. Give the active player 1 red mana. Assert that the protected creature is NOT offered as a legal target for the {R},{T} fight ability.

