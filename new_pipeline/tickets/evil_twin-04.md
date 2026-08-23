---
id: evil_twin-04
status: fixed
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
fixed_sha: ac58079cd610fdd6b957d0dadbe3f542dfd7779c
fixed_at: 2026-08-23T23:16:14Z
test_file: mtg-engine/tests/zone_change_resets_object.rs
fix_note: cluster fix: move_object now restores printed identity (card_id/name/base P-T) and clears attached_to_player on leaving the battlefield (CR 400.7)
---

## Audit Finding

**Oracle text:**
> Evil Twin copies exactly what was printed on the original creature (unless that creature is copying something else or is a token; see below) and it gains the activated ability. It doesn't copy whether that creature is tapped or untapped, whether it has any counters on it or any non-copy effects that have changed its power, toughness, types, color, or so on.

**Code:**
> obj.name.clone_from(&name);
obj.power = power;
obj.toughness = toughness;
obj.card_id = card_id;
obj.keywords = keywords;
obj.card_types = card_types;
obj.subtypes = subtypes;
obj.colors = colors;

**Description:**
The `CopyCreature` handler mutates `name`, `card_id`, `keywords`, `subtypes`, `card_types`, and `colors` directly on the object. The `move_object` cleanup block (state.rs:586-608) does not reset any of these fields when the permanent leaves the battlefield — it only resets `name`/`keywords`/`subtypes` for the `is_transformed` DFC case. For Evil Twin, `is_transformed` is always false, so none of the copied fields are restored. Per CR 400.7, when Evil Twin leaves and re-enters the battlefield it is a new object with no memory of the previous copy. In practice: (a) Evil Twin in the graveyard carries the copied creature's name (wrong identity); (b) when reanimated, `card_id` still points to the copied creature, so the ETB trigger dispatch fires that creature's ETB handler instead of Evil Twin's — the copy choice is never presented, and Evil Twin stays as a permanent copy of the original creature with no `is_evil_twin` marker (cleared by `card_state.clear()` on re-entry) and therefore no destroy ability.

**Engine path:** mtg-engine/src/state.rs:586

**Required check:** 8a

## Tests

### evil_twin_copy_name_reset_in_graveyard
Scenario: Evil Twin copies Grizzly Bears, then dies; its name in the graveyard should be 'Evil Twin' (a new object per CR 400.7) but is currently 'Grizzly Bears'.

### evil_twin_copy_choice_presented_after_reanimate
Scenario: Evil Twin copies Grizzly Bears, dies, and is then reanimated; Evil Twin's ETB copy-choice should fire again on re-entry, but currently the ETB fires for Grizzly Bears' behavior (wrong card_id) and the copy choice is never presented.

