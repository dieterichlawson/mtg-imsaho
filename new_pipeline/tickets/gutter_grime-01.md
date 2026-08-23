---
id: gutter_grime-01
status: new
card: Gutter Grime
audit_run_id: 2026-04-19-gutter_grime-audit
audit_model: sonnet
audit_tokens: 42107
audit_duration: 797
---

## Audit Finding

**Oracle text:**
> Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."

**Code:**
> let simultaneously_dead: Vec<ObjectId> = events.iter().filter_map(|e| {
    if let GameEvent::CreatureDied { object, .. } = e {
        Some(*object)
    } else {
        None
    }
}).collect();
let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
    .filter(|o| o.id != dead_id &&
        (o.zone == Zone::Battlefield || simultaneously_dead.contains(&o.id)))
    .map(|o| (o.id, o.card_id, o.controller))
    .collect();

**Description:**
When Gutter Grime is destroyed in the same event batch as a nontoken creature it controls (e.g., a 'destroy all permanents' effect, or simultaneous SBA processing), its AnyCreatureDies trigger is never created. The `simultaneously_dead` list is populated only from `GameEvent::CreatureDied` events; the `destroy()` function in destruction.rs only emits `CreatureDied` for objects where `power.is_some()` (i.e., creatures). Gutter Grime is an Enchantment with no power, so no `CreatureDied` event is emitted for it when it is destroyed. After the batch is processed, Gutter Grime's zone is `Zone::Graveyard`, which fails the `o.zone == Zone::Battlefield` check, and its ObjectId is absent from `simultaneously_dead`, which fails the second condition. The trigger is silently dropped and Gutter Grime never receives a slime counter or creates an Ooze token for that creature's death.

**Engine path:** mtg-engine/src/triggers.rs:647

**Required check:** 8b

**Affected cards:**
- Gutter Grime

## Tests

### gutter_grime_trigger_missed_on_simultaneous_destruction
Scenario: A 'destroy all permanents' effect destroys both Gutter Grime and a nontoken creature the controlling player owns; Gutter Grime should have gained a slime counter and created an Ooze token before leaving the battlefield, but neither happens.

