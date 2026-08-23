---
id: evil_twin-02
status: fixed
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
fixed_sha: 778ed4738894357d762d118ea082f892dcb0d2c4
fixed_at: 2026-08-23T23:24:11Z
test_file: mtg-engine/tests/copy_effects.rs
fix_note: the copied creature's ETB abilities now trigger for the copy (CR 614.12), without re-emitting the entering event
---

## Audit Finding

**Oracle text:**
> Any enters-the-battlefield abilities of the copied creature will trigger when Evil Twin enters the battlefield. Any "as [this creature] enters the battlefield" or "[this creature] enters the battlefield with" abilities of the chosen creature will also work.

**Code:**
> if let Some(obj) = state.get_object_mut(*source_id) {
    obj.name.clone_from(&name);
    obj.power = power;
    obj.toughness = toughness;
    obj.card_id = card_id;
    obj.keywords = keywords;
    obj.card_types = card_types;
    obj.subtypes = subtypes;
    obj.colors = colors;
    obj.card_state.insert("is_evil_twin".into(), ObjectId(1));
}
state.log(LogLevel::Event,
    format!("Evil Twin enters as a copy of {}", state.obj_name(*target_id)));

**Description:**
The copy is implemented as a pending effect (`CopyCreature`) that resolves AFTER Evil Twin has already entered the battlefield and the `EnteredBattlefield` event has fired. The ETB trigger dispatch (triggers.rs:537-546) captures `card_id` at entry time (Evil Twin's own card_id), and the only ETB trigger created is for Evil Twin itself (the copy choice). When the `CopyCreature` effect resolves later, characteristics change but no new `EnteredBattlefield` event is emitted. The copied creature's ETB triggered abilities therefore never fire. Per CR 614.12, the copy is supposed to be a replacement effect applied as the permanent enters — meaning the permanent enters already bearing the copied characteristics, and the ETB event fires once with those characteristics so that the copied creature's ETB abilities trigger. The current architecture inverts this order: Evil Twin enters as itself, fires its own ETB, then the characteristics are swapped by a pending effect resolution, and the copied creature's ETBs are permanently missed.

**Engine path:** mtg-engine/src/engine.rs:3761

**Required check:** 8j

## Tests

### evil_twin_copy_etb_triggers_from_copied_creature
Scenario: Evil Twin copies Fiend Hunter (which has an ETB that exiles a creature); Fiend Hunter's ETB triggered ability should fire and prompt the player to exile a creature, but currently does not.

### evil_twin_copy_enters_with_abilities_from_copied_creature
Scenario: Evil Twin copies a creature with an 'as this creature enters the battlefield with N counters' ability; Evil Twin should enter with those counters, but currently does not.

