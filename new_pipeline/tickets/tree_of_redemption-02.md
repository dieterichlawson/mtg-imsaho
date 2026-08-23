---
id: tree_of_redemption-02
status: fixed
card: Tree of Redemption
audit_run_id: 2026-04-19-tree_of_redemption-audit
audit_model: sonnet
audit_tokens: 14966
audit_duration: 349
fixed_sha: ac58079cd610fdd6b957d0dadbe3f542dfd7779c
fixed_at: 2026-08-23T23:16:14Z
test_file: mtg-engine/tests/zone_change_resets_object.rs
fix_note: cluster fix: move_object now restores printed identity (card_id/name/base P-T) and clears attached_to_player on leaving the battlefield (CR 400.7)
---

## Audit Finding

**Oracle text:**
> {T}: Exchange your life total with this creature's toughness.

**Code:**
> if let Some(obj) = state.get_object_mut(object_id) {
    obj.toughness = Some(current_life);
}

**Description:**
The ability directly sets `obj.toughness` (the base toughness) to the former life total. The `move_object` cleanup block in state.rs (lines 599–621) resets many battlefield-specific fields when an object leaves the battlefield, but `obj.toughness` is not among them. If the Tree later dies or is otherwise moved off the battlefield, the graveyard object retains the modified toughness value. When the Tree subsequently re-enters the battlefield (e.g. via reanimation), the entering-battlefield block (state.rs:631–634) only clears `card_state` and sets `summoning_sick`; it does not reset `obj.toughness` to the registry value of 13. The reanimated Tree therefore enters with whatever toughness the exchange previously set, violating CR 400.7 (a zone-changed object becomes a new object with no memory of its previous existence).

**Engine path:** mtg-engine/src/state.rs:599

**Required check:** 8a

## Tests

### tree_toughness_resets_after_death_and_reanimate
Scenario: Tree's ability exchanges toughness to 7 (life was 7); Tree is then destroyed and reanimated; the reanimated Tree should enter with toughness 13, not 7.

