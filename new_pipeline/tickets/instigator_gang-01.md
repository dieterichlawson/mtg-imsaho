---
id: instigator_gang-01
status: fixed
card: Instigator Gang
audit_run_id: 2026-04-19-instigator_gang-audit
audit_model: sonnet
audit_tokens: 32082
audit_duration: 573
fixed_sha: 28755d7786c3882a8061e402a59c15fd2378da86
fixed_at: 2026-08-23T17:03:38Z
test_file: mtg-engine/tests/intervening_if.rs
fix_note: back face was missing its TriggerKind::Upkeep entry, so Wildblood Pack could never transform back
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Code:**
> triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::AnyCreatureAttacks,
        description: "attacking creatures you control get +3/+0".into(),
    target_requirement: None,
    },
],

**Description:**
The back face (Wildblood Pack) is missing a `TriggerKind::Upkeep` entry in its `back_face_data().triggered_abilities` vec. The upkeep trigger dispatch in `triggers.rs` uses `face_trigger_description`, which reads only the currently-visible face's `triggered_abilities`. When the card is transformed (`is_transformed = true`), `face_trigger_description` looks up `back_face_data().triggered_abilities` — which contains only `AnyCreatureAttacks` and no `Upkeep` entry. The returned description is empty, so the dispatch loop skips creating the upkeep trigger entirely, and `on_upkeep` is never called for Wildblood Pack. As a result, Wildblood Pack can never transform back to Instigator Gang regardless of how many spells are cast. Compare with Village Ironsmith/Ironfang (`village_ironsmith.rs:65-71`) and Mayor of Avabruck/Howlpack Alpha (`mayor_of_avabruck.rs:87-99`), both of which correctly include `TriggerKind::Upkeep` in their back-face `triggered_abilities`.

**Engine path:** mtg-engine/src/cards/isd/instigator_gang.rs:71

**Required check:** 8b

**Affected cards:**
- Instigator Gang / Wildblood Pack

## Tests

### wildblood_pack_never_transforms_back
Scenario: With Instigator Gang already transformed to Wildblood Pack, a player casts two spells during their turn; at the beginning of the following upkeep Wildblood Pack fails to transform back to Instigator Gang.

### wildblood_pack_opponent_casts_two_no_transform_back
Scenario: The opponent casts two spells on their turn while the player controls Wildblood Pack; at the beginning of the next upkeep (opponent's or player's) the card incorrectly remains as Wildblood Pack.

