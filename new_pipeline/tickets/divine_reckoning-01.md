---
id: divine_reckoning-01
status: fixed
card: Divine Reckoning
audit_run_id: 2026-04-19-divine_reckoning-audit
audit_model: sonnet
audit_tokens: 33181
audit_duration: 977
fixed_sha: 677e197
fixed_at: 2026-08-24T01:12:48Z
test_file: mtg-engine/tests/simultaneous_events.rs
fix_note: destruction::try_destroy_all decides for every permanent against the state before any of them died (CR 700.2c).
---

## Audit Finding

**Oracle text:**
> Each player chooses a creature they control. Destroy the rest.

**Code:**
> let all_creatures: Vec<ObjectId> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
                    .map(|o| o.id)
                    .collect();
                for cid in all_creatures {
                    if !kept.contains(&cid) {
                        crate::destruction::try_destroy(state, cid, registry);
                    }
                }

**Description:**
"Destroy the rest" must destroy all unchosen creatures simultaneously, but the KeepOneDestroyRest handler (engine.rs:3808–3816) and the no-pending-players branch of on_resolve (divine_reckoning.rs:74–82) both call try_destroy in a sequential loop. try_destroy immediately moves each creature to Zone::Graveyard before the next iteration runs. This means conditional indestructibility is re-evaluated between destructions: for example, if both Angelic Overseer and the last Human under a player's control are unchosen, the correct simultaneous rule is that the Human is alive at the moment of destruction, so Angelic Overseer is indestructible and survives. But if the sequential loop processes the Human first, Angelic Overseer loses its indestructible condition and is then also destroyed — the wrong outcome. The same bug affects any card whose indestructibility, toughness, or other characteristics depend on the number or presence of other creatures on the battlefield at the moment of destruction.

**Engine path:** mtg-engine/src/engine.rs:3808

**Affected cards:**
- Divine Reckoning

## Tests

### conditional_indestructible_survives_when_last_protector_dies_simultaneously
Scenario: Player A keeps one creature; the unchosen creatures include Angelic Overseer and the last Human under player A's control — the Overseer should survive because the Human is alive at the moment simultaneous destruction occurs, but the sequential loop processes the Human first, stripping the condition before the Overseer's destruction check.

### all_unchosen_creatures_destroyed_without_intermediate_state_changes
Scenario: With three unchosen creatures that have no conditional interactions, all three should be destroyed in the same logical step with no creature seeing the others as already dead when the destruction check runs — the sequential loop violates this invariant for any state-sensitive indestructible check.

