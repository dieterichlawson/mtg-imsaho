---
id: runechanter_s_pike-01
status: new
card: Runechanter's Pike
audit_run_id: 2026-04-19-runechanter_s_pike-audit
audit_model: sonnet
audit_tokens: 46835
audit_duration: 2354
---

## Audit Finding

**Oracle text:**
> Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.

**Code:**
> fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
    let obj = state.get_object(object_id)?;
    if obj.zone != Zone::Battlefield {
        return None;
    }
    let controller = obj.controller;
    let count = i32::try_from(state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
        .filter(|o| {
            o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery)
        })
        .count()).unwrap_or(i32::MAX);
    Some((count, 0))
}

**Description:**
The Pike's `dynamic_pt` returns `Some((count, 0))` unconditionally whenever the Pike is on the battlefield. The engine calls this method from two distinct paths: (1) correctly, from `continuous_pt_mods` (state.rs:984-988) when computing the attached creature's P/T — where `source.id` is the Pike's ObjectId; and (2) incorrectly, from `effective_power`'s self-check branch (state.rs:1092), which is intended only for creatures with characteristic-defining abilities — where `id` is also the Pike's ObjectId. In path (2) the `dynamic_pt` runs the same graveyard count and returns `Some((count, 0))`, so `effective_power(pike_id)` returns `Some(count)` and `effective_toughness(pike_id)` returns `Some(0)`. This populates `PermanentView.effective_power` and `PermanentView.effective_toughness` with non-None values for an artifact that has no P/T at all. AI and LLM players that use `effective_power.is_some()` as a creature indicator will misidentify the Pike as a creature; those reading `effective_toughness = Some(0)` may reason that the Pike is at SBA risk. The oracle text grants P/T only to the EQUIPPED CREATURE, never to the equipment itself. The fix belongs in `state.rs`: guard the self-check branch in `effective_power`/`effective_toughness` with `obj.power.is_some()` so Equipment and Auras that implement `dynamic_pt` for their contribution to attached objects are not mistakenly evaluated for their own P/T. Wreath of Geists (isd/wreath_of_geists.rs) has the identical structural pattern and the same bug.

**Engine path:** mtg-engine/src/state.rs:1090-1096

**Affected cards:**
- Wreath of Geists

## Tests

### pike_effective_power_is_none_not_count
Scenario: With Pike equipped to a creature and two instants in the controller's graveyard, state.effective_power(pike_id, registry) should return None, not Some(2).

### pike_effective_toughness_is_none_not_zero
Scenario: With Pike on the battlefield (equipped or unequipped), state.effective_toughness(pike_id, registry) should return None, not Some(0).

### pike_permanent_view_effective_pt_is_none
Scenario: The PermanentView for the Pike on the battlefield must have effective_power = None and effective_toughness = None regardless of graveyard contents.

