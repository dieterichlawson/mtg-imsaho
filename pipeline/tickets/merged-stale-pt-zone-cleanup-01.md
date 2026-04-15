---
id: merged-stale-pt-zone-cleanup-01
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: tree_of_redemption-01, moonmist-01
---

# move_object cleanup does not clear stale obj.power/obj.toughness (CR 400.7)

## Description
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The engine's `move_object` cleanup block (state.rs:572-583) clears `tapped`, `summoning_sick`, `damage_marked`, `counters`, `is_transformed`, etc. — but does NOT clear `obj.power` or `obj.toughness`. Cards that modify these fields at runtime (Tree of Redemption's toughness exchange, Moonmist's manual P/T mutation during transform) leave stale values that persist after the creature leaves the battlefield. When the creature returns, `effective_power` (state.rs:1057) and `effective_toughness` (state.rs:1118) fall through to the stale `obj.power`/`obj.toughness` values when `dynamic_pt()` returns `None`, showing incorrect stats.

## Engine path
- state.rs:572-583 (move_object cleanup — power/toughness not cleared)
- state.rs:1057 (effective_power falls through to stale obj.power)
- state.rs:1118 (effective_toughness falls through to stale obj.toughness)

## Tests

### tree_of_redemption_toughness_resets_after_zone_change
Source ticket: tree_of_redemption-01
Implementation: (not yet written)
Scenario: Tree of Redemption (0/13) exchanges toughness with a player at 20 life, becoming 0/20. Bounce Tree to hand and replay it. Assert effective_toughness is 13 (base), not 20 (stale exchange value). Currently fails because obj.toughness retains the exchanged value across the zone change.

### moonmist_transform_stale_pt_after_bounce
Source ticket: moonmist-01
Implementation: (not yet written)
Scenario: Put Gatstaf Shepherd (2/2 front / 3/3 back) on the battlefield. Cast Moonmist to transform it. Verify effective_power is 3. Bounce Gatstaf Shepherd to hand. Replay it. Assert effective_power is 2 and effective_toughness is 2. Currently fails because Moonmist wrote obj.power=3 directly and the value persists through zone change.

