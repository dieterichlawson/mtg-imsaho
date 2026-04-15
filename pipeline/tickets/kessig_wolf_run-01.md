---
id: kessig_wolf_run-01
status: deduped
card: Kessig Wolf Run
card_file: mtg-engine/src/cards/isd/kessig_wolf_run.rs
created: 2026-04-14T21:29:34Z
audit_run_id: 2026-04-14-kessig_wolf_run-audit
audit_model: opus
audit_tokens: 14580
audit_duration: 309
deduped_into: merged-temp-effect-zone-persist-01
---

## Audit Finding

**Oracle text:**
> {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.

**Code:**
> `state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT { target: *target_id, power_mod: x, toughness_mod: 0 });` — kessig_wolf_run.rs:70-74
> `state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantKeyword { target: *target_id, keyword: Keyword::Trample });` — kessig_wolf_run.rs:76-79

**Description:**
The +X/+0 and trample effects are stored as `TemporaryEffect` entries keyed by the target's `ObjectId`. When the target creature leaves the battlefield (blink, bounce, sacrifice), `move_object` (state.rs:572-583) does NOT remove `until_end_of_turn` entries that reference the departing object. Since the engine reuses the same `ObjectId` across zone changes (only incrementing `zone_change_count`), if the creature returns to the battlefield (replayed from hand or blinked back), the stale `TemporaryEffect` entries still match its `ObjectId`. Both `effective_power` (state.rs:1072-1076) and `has_keyword` (state.rs:1253-1256) unconditionally apply matching `TemporaryEffect` entries without checking `zone_change_count`. Per CR 400.7, a zone change creates a new object with no memory of prior existence — the +X/+0 and trample should not persist.

**Engine path:**
- state.rs:572-583 (move_object cleanup — no TemporaryEffect pruning)
- state.rs:1072-1076 (effective_power — no zone_change_count check)
- state.rs:1253-1256 (has_keyword — no zone_change_count check)
- kessig_wolf_run.rs:70-79 (effect application)

**Required check:** 8a, 8h

**Affected cards:**
- Kessig Wolf Run
- Every card that pushes TemporaryEffect entries targeting another object (Giant Growth, Fires of Undeath, Moment of Heroism, Ranger's Guile, etc.)
