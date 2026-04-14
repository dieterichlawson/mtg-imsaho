---
id: morkrut_banshee-02
status: new
card: Morkrut Banshee
card_file: mtg-engine/src/cards/isd/morkrut_banshee.rs
created: 2026-04-14T21:54:58Z
audit_run_id: 2026-04-14-morkrut_banshee-audit
audit_model: opus
audit_tokens: 16831
audit_duration: 1625
---

## Audit Finding

**Oracle text:**
> target creature gets -4/-4 until end of turn.

**Code:**
> `engine.rs:3528-3535`: DebuffUntilEOT handler pushes `TemporaryEffect::ModifyPT { target: *id, power_mod: *power, toughness_mod: *toughness }` to `state.until_end_of_turn`.
> `state.rs:572-583`: `move_object` cleanup block does NOT remove TemporaryEffect entries from `until_end_of_turn` when an object changes zones.
> `state.rs:1074`: `effective_power` applies `ModifyPT` by matching `*target == id` with no zone-change-count guard.

**Description:**
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. If a creature receives -4/-4 from Morkrut Banshee, then changes zones (dies, gets exiled, bounced) and returns to the battlefield in the same turn, the `TemporaryEffect::ModifyPT` entry still targets it by ObjectId (which the engine reuses) and is still present in `until_end_of_turn` (not cleaned up on zone change). The `effective_power`/`effective_toughness` functions apply it unconditionally when the target ID matches. The reanimated creature incorrectly has -4/-4 applied — if its base toughness is 4 or less, it dies again immediately from SBAs. The fix would be to either store and check `zone_change_count` in the TemporaryEffect, or remove TemporaryEffect entries targeting an object when it changes zones.

**Engine path:**
- engine.rs:3530-3533 (TemporaryEffect::ModifyPT creation, no zone_change_count)
- state.rs:572-583 (move_object cleanup, does not touch until_end_of_turn)
- state.rs:1074-1076 (effective_power, no zone-change guard)

**Required check:** 8h (continuous effect duration)

**Affected cards:**
- Morkrut Banshee
- All cards using TemporaryEffect::ModifyPT (engine-wide)

