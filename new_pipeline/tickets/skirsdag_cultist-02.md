---
id: skirsdag_cultist-02
status: fixed
card: Skirsdag Cultist
audit_run_id: 2026-04-19-skirsdag_cultist-audit
audit_model: sonnet
audit_tokens: 29132
audit_duration: 1795
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: AnyTarget branch uses state.has_card_type for planeswalkers; regression coverage present
---

## Audit Finding

**Oracle text:**
> This creature deals 2 damage to any target.

**Code:**
> TargetRequirement::AnyTarget => {
    let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| o.power.is_some() || o.card_types.contains(&CardType::Planeswalker))
        .filter(|o| can_be_targeted(state, o.id, controller, registry))

**Description:**
The AnyTarget branch in generate_ability_targets (engine.rs:2055-2058) filters battlefield permanents with o.power.is_some() || o.card_types.contains(&CardType::Planeswalker). Because create_object initialises card_types: Vec::new() for all non-token permanents, o.card_types.contains(&CardType::Planeswalker) is always false for non-token planeswalkers, making them invisible to the filter. The PlayerOrPlaneswalker branch at engine.rs:2043-2046 already has the correct registry fallback (registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Planeswalker))). The AnyTarget branch in generate_ability_targets is missing this second clause. Per CR 115.4, 'any target' includes planeswalkers; Skirsdag Cultist's activated ability cannot target any non-token planeswalker on the battlefield despite oracle text permitting it.

**Engine path:** mtg-engine/src/engine.rs:2057

**Required check:** 8f

## Tests

### skirsdag_cultist_cannot_target_nontoken_planeswalker
Scenario: A non-token planeswalker is on the battlefield; Skirsdag Cultist's activated ability target list should include it as a valid 'any target' option, but it is absent because the AnyTarget filter lacks a registry fallback for non-token planeswalkers.

