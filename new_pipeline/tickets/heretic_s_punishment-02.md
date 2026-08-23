---
id: heretic_s_punishment-02
status: new
card: Heretic's Punishment
audit_run_id: 2026-04-19-heretic_s_punishment-audit
audit_model: sonnet
audit_tokens: 23651
audit_duration: 422
---

## Audit Finding

**Oracle text:**
> Choose any target

**Code:**
> TargetRequirement::AnyTarget => {
    let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| o.power.is_some() || o.card_types.contains(&CardType::Planeswalker))
        .filter(|o| can_be_targeted(state, o.id, controller, registry))
        .map(|o| Target::Object(o.id))
        .filter(|t| behavior.is_valid_target(state, controller, t, registry))
        .collect();
    ...
}

**Description:**
Per CR 115.4a, 'any target' includes creatures, players, planeswalkers, and battles. The `AnyTarget` branch in `generate_ability_targets` (engine.rs:2055–2070) filters battlefield permanents by `o.power.is_some() || o.card_types.contains(&CardType::Planeswalker)`. However, per the established engine insight, non-token permanents have `card_types: Vec::new()` on the battlefield object; the registry is the authoritative source for card type. This means `o.card_types.contains(&CardType::Planeswalker)` is always false for non-token planeswalkers such as Liliana of the Veil or Garruk Relentless. The `PlayerOrPlaneswalker` branch (engine.rs:2043–2046) already contains the correct fix — it adds `|| registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Planeswalker))` — but `AnyTarget` was not updated with the same fallback. As a result, Heretic's Punishment (and every other card declaring `TargetRequirement::AnyTarget`) cannot present a non-token planeswalker as a valid target even though the oracle text requires it.

**Engine path:** mtg-engine/src/engine.rs:2057

**Required check:** 8f

**Affected cards:**
- Burning Vengeance
- Pitchburn Devils
- Skirsdag Cultist
- Blazing Torch
- Lightning Bolt
- Brimstone Volley
- Geistflame
- Devil's Play

## Tests

### any_target_includes_planeswalker
Scenario: A Liliana of the Veil (or Garruk Relentless) is on the battlefield; when Heretic's Punishment's activated ability is offered, the planeswalker should appear in the target list but currently does not.

