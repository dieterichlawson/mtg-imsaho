---
id: charmbreaker_devils-02
status: deduped
card: Charmbreaker Devils
card_file: mtg-engine/src/cards/isd/charmbreaker_devils.rs
created: 2026-04-14T21:23:50Z
audit_run_id: 2026-04-14-charmbreaker_devils-audit
audit_model: opus
audit_tokens: 23951
audit_duration: 476
deduped_into: merged-temp-effect-zone-persist-01
---

## Audit Finding

**Oracle text:**
> Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.

**Code:**
> `charmbreaker_devils.rs:94-98`: `state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT { target: self_id, power_mod: 4, toughness_mod: 0, });`
> `state.rs:1074`: `TemporaryEffect::ModifyPT { target, power_mod, .. } if *target == id => { power += power_mod; }` — applies the modifier whenever target ObjectId matches, with no zone_change_count check.
> `state.rs:572-583`: `move_object` cleanup does not remove `until_end_of_turn` entries targeting the leaving object.

**Description:**
If Charmbreaker Devils receives +4/+0 from its second ability, then leaves the battlefield and re-enters in the same turn (e.g., bounced to hand and recast, or destroyed and reanimated), the +4/+0 modifier incorrectly persists. The engine reuses the same ObjectId across zone changes, and `effective_power()` (state.rs:1074) matches `TemporaryEffect::ModifyPT` by ObjectId alone without checking whether the object has changed zones since the effect was created. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence; the old +4/+0 effect should not apply to the new permanent. The `move_object` cleanup block (state.rs:572-583) does not remove `until_end_of_turn` entries for the leaving object, and the `TemporaryEffect` struct does not track `zone_change_count` to detect staleness.

**Engine path:**
- charmbreaker_devils.rs:94-98 (effect creation)
- state.rs:1074 (effect application without zone-change tracking)
- state.rs:572-583 (zone-change cleanup omits until_end_of_turn pruning)

**Required check:** 8a, 8h

**Affected cards:**
- Charmbreaker Devils
- All cards that use `TemporaryEffect::ModifyPT` (engine-wide: any +N/+N until end of turn effect)
