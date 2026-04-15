---
id: trepanation_blade-03
status: new
card: Trepanation Blade
card_file: mtg-engine/src/cards/isd/trepanation_blade.rs
created: 2026-04-15T03:52:30Z
audit_run_id: 2026-04-14-trepanation_blade-audit
audit_model: opus
audit_tokens: 28121
audit_duration: 580
---

## Audit Finding

**Oracle text:**
> The creature gets +1/+0 until end of turn for each card revealed this way.

**Code:**
> `state.rs:1074`:
> ```rust
> TemporaryEffect::ModifyPT { target, power_mod, .. } if *target == id => {
>     power += power_mod;
> }
> ```
> `trepanation_blade.rs:102-108`:
> ```rust
> state.until_end_of_turn.push(
>     crate::state::TemporaryEffect::ModifyPT {
>         target: creature_id,
>         power_mod: cards_milled,
>         toughness_mod: 0,
>     }
> );
> ```

**Description:**
The `TemporaryEffect::ModifyPT` entry targets the creature by `ObjectId`. If the creature leaves the battlefield and re-enters (e.g., via a blink effect like Cloudshift) during the same turn, the engine reuses the same `ObjectId` with an incremented `zone_change_count`. The `effective_power` calculation at state.rs:1074 matches `ModifyPT` entries purely by `ObjectId` — it does not check `zone_change_count` or whether the creature has undergone a zone change since the effect was created. Additionally, `move_object` at state.rs:564-583 does not remove stale `until_end_of_turn` entries when an object leaves the battlefield. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The +N/+0 buff should not apply to the "new" creature after re-entry. This is a known engine-wide issue affecting all `TemporaryEffect::ModifyPT` users.

**Engine path:**
- state.rs:1074 (ModifyPT matching lacks zone_change_count check)
- state.rs:564-583 (move_object cleanup does not remove stale until_end_of_turn entries)

**Required check:** 8h, 8a

**Affected cards:**
- Trepanation Blade
- All cards that push TemporaryEffect::ModifyPT (Giant Growth, Moment of Heroism, Hamlet Captain, etc.)

## Tests

### temporary_buff_does_not_persist_through_blink
Source ticket: (new)
Implementation: (not yet written)
Scenario: Equip Trepanation Blade to a 2/2 creature. Set up defender's library so 3 cards are milled (2 nonlands + 1 land). Trigger the blade's ability so the creature gets +3/+0 (effective power 5). Then blink the creature (move to exile, then back to battlefield). Assert that the creature's effective power is 2 (base), not 5 — the buff should not survive the zone change.

