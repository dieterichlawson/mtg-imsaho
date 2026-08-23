---
id: splinterfright-02
status: new
card: Splinterfright
audit_run_id: 2026-04-19-splinterfright-audit
audit_model: sonnet
audit_tokens: 27529
audit_duration: 462
---

## Audit Finding

**Oracle text:**
> Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.

**Code:**
> let controller = state.get_object(object_id)?.controller;

**Description:**
Per CR 112.8, a card not on the stack or battlefield is controlled by its owner. The engine's `move_object` does not reset `obj.controller` to `obj.owner` when a permanent leaves the battlefield (see 'Controller field not reset to owner on zone change affects CDAs' insight). Splinterfright's `dynamic_pt` reads `obj.controller` to determine whose graveyard to count, then passes that value to `state.objects_in_zone(Zone::Graveyard, controller)`. That function filters by `obj.owner == player` for graveyard zones — so the player argument must be the owner, not a potentially-stale controller. After a temporary control effect such as Act of Treason, Splinterfright's `controller` field in the graveyard still refers to the opponent (Player B). `objects_in_zone(Zone::Graveyard, Player B)` then returns objects owned by the opponent, causing Splinterfright to count the opponent's creature cards instead of its own owner's. The official ruling ([2025-01-24]) states 'If Splinterfright is in your graveyard, it will count itself' — implying 'your' refers to the card's owner in the graveyard. The fix is to read `obj.owner` rather than `obj.controller`.

**Engine path:** mtg-engine/src/cards/isd/splinterfright.rs:44

**Required check:** 8j

**Affected cards:**
- Boneyard Wurm

## Tests

### cda_uses_owner_graveyard_after_temporary_control
Scenario: Opponent steals Splinterfright via a control-change effect; Splinterfright dies and moves to its owner's graveyard which already contains 2 other creature cards; verify Splinterfright's CDA counts 3 (itself + 2 owner cards), not 0 from the opponent's empty graveyard.

