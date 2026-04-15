---
id: sturmgeist-01
status: closed-duplicate
card: Sturmgeist
card_file: mtg-engine/src/cards/isd/sturmgeist.rs
created: 2026-04-15T03:46:37Z
audit_run_id: 2026-04-14-sturmgeist-audit
audit_model: opus
audit_tokens: 22153
audit_duration: 451
duplicate_of: merged-zone-cleanup-characteristics-02
---

## Audit Finding

**Oracle text:**
> Sturmgeist's power and toughness are each equal to the number of cards in your hand.

**Code:**
> `let controller = state.get_object(object_id)?.controller;` (sturmgeist.rs:41)
> `let hand_size = ... state.objects_in_zone(Zone::Hand, controller).len() ...` (sturmgeist.rs:42)

**Description:**
Per CR 112.8, a card not on the stack or battlefield is controlled by its owner. When Sturmgeist leaves the battlefield, `move_object` (state.rs:572-583) does not reset the `controller` field to `owner`. If Sturmgeist was stolen (e.g., via Mind Control or Act of Treason) and then dies or is exiled, `obj.controller` retains the thief's player ID. The `dynamic_pt` CDA then calls `objects_in_zone(Zone::Hand, controller)` with the thief's ID, counting cards in the thief's hand instead of the owner's hand. This violates CR 112.8 and produces an incorrect P/T for any subsequent check of Sturmgeist's characteristics in a non-battlefield zone (e.g., Corpse Lunge exiling it from graveyard, or another effect referencing its power in the graveyard).

**Engine path:**
- state.rs:572-583 — zone-change cleanup block does not include `obj.controller = obj.owner;`
- sturmgeist.rs:41 — `dynamic_pt` reads `controller` which may be stale

**Required check:** 8a

**Affected cards:**
- Sturmgeist
- Boneyard Wurm (same pattern: dynamic_pt reads controller for graveyard count)
- Geist-Honored Monk (same pattern: dynamic_pt reads controller for creature count)
- Any future CDA creature using `obj.controller` instead of `obj.owner` off-battlefield

## Tests

### sturmgeist_cda_uses_owner_hand_after_stolen_death
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A (P0) owns Sturmgeist. Set Sturmgeist's controller to P1 (simulating a steal effect). Give P0 3 cards in hand, P1 5 cards in hand. Move Sturmgeist to graveyard. Assert `effective_power(sturmgeist, &registry) == 3` (owner P0's hand size), not 5 (thief P1's hand size).
