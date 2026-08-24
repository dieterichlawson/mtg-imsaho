---
id: skirsdag_high_priest-01
status: fixed
card: Skirsdag High Priest
audit_run_id: 2026-04-19-skirsdag_high_priest-audit
audit_model: sonnet
audit_tokens: 37575
audit_duration: 1083
fixed_sha: 74c31e5
fixed_at: 2026-08-24T00:53:36Z
test_file: mtg-engine/tests/tap_cost_legality.rs
fix_note: {T}-cost legality centralized in GameState::can_pay_tap_cost + engine::available_mana_abilities; the card-level guard that shadowed it (and dropped the haste exception) is gone.
---

## Audit Finding

**Oracle text:**
> {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.

**Code:**
> if obj.zone != Zone::Battlefield || obj.tapped || obj.summoning_sick {
    return vec![];
}

**Description:**
The card-level guard in `activated_abilities` checks `obj.summoning_sick` unconditionally, with no haste exception. Per CR 302.6, a creature with haste can pay {T} costs even when it has summoning sickness. If Skirsdag High Priest gains haste from another permanent (e.g., Lightning Greaves, Fires of Yavimaya), `activated_abilities` still returns an empty vector, so the ability never appears in legal actions. The engine's own summoning-sickness guard in `legal_actions` (which correctly consults `state.has_keyword(obj.id, Keyword::Haste, registry)` before skipping) is never reached because the card-level check pre-empts it by returning an empty ability list first.

**Engine path:** mtg-engine/src/cards/isd/skirsdag_high_priest.rs:35

**Required check:** 8c

## Tests

### skirsdag_high_priest_with_haste_can_activate_while_summoning_sick
Scenario: Skirsdag High Priest enters the battlefield this turn (summoning_sick=true), gains haste from another permanent, and a creature died this turn — the ability should appear in legal_actions but currently does not.

