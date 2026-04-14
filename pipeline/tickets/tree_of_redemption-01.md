---
id: tree_of_redemption-01
status: new
card: Tree of Redemption
card_file: mtg-engine/src/cards/isd/tree_of_redemption.rs
created: 2026-04-14T21:48:34Z
audit_run_id: 2026-04-14-tree_of_redemption-audit
audit_model: opus
audit_tokens: 9193
audit_duration: 1139
---

## Audit Finding

**Oracle text:**
> {T}: Exchange your life total with this creature's toughness.

**Code:**
> `state.rs:572-583`: The `move_object` cleanup block clears `tapped`, `summoning_sick`, `damage_marked`, `counters`, `is_transformed`, etc. but does NOT clear `toughness` (or `power`).
> `tree_of_redemption.rs:72`: `obj.toughness = Some(current_life);`

**Description:**
When Tree of Redemption exchanges its toughness with the player's life total, the base toughness is modified by writing directly to `obj.toughness`. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. If Tree is later bounced, killed, or exiled and then returns to the battlefield (e.g., via reanimation), it should return as a 0/13 creature. Instead, it retains whatever toughness value was written during the exchange (e.g., 0/20 after a single exchange at 20 life). The `effective_toughness` function (state.rs:1100) reads from `obj.toughness`, so the stale value is used for all game computations.

**Engine path:**
- state.rs:572-583 (move_object cleanup block — toughness not cleared)
- state.rs:1100-1122 (effective_toughness reads obj.toughness)
- tree_of_redemption.rs:72 (writes obj.toughness)

**Required check:** 8a

**Affected cards:**
- Tree of Redemption
- Any card that modifies `obj.toughness` or `obj.power` at runtime (engine-wide)

