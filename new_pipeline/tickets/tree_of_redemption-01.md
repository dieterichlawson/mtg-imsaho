---
id: tree_of_redemption-01
status: new
card: Tree of Redemption
audit_run_id: 2026-04-19-tree_of_redemption-audit
audit_model: sonnet
audit_tokens: 14966
audit_duration: 349
---

## Audit Finding

**Oracle text:**
> [2018-03-16] If Tree of Redemption isn't on the battlefield when its activated ability resolves, the exchange can't happen and the ability will have no effect.

**Code:**
> let controller = match state.get_object(object_id) {
    Some(o) => o.controller,
    None => return,
};

**Description:**
The ability handler checks only whether the source object exists in the game state (`get_object` returns Some), not whether it is still on the battlefield. Activated abilities remain on the stack when their source leaves the battlefield, so if the Tree is destroyed, bounced, or exiled in response to the activation, `get_object(object_id)` still finds the object in the graveyard (or hand/exile), the zone guard silently passes, `effective_toughness` returns the graveyard object's stored toughness, and the life exchange executes. Per ruling 1, the ability must have no effect in this case. The fix is to check `o.zone == Zone::Battlefield` immediately after retrieving the object and return early if not.

**Engine path:** mtg-engine/src/cards/isd/tree_of_redemption.rs:52

**Required check:** 8j

## Tests

### tree_destroyed_in_response_no_exchange
Scenario: Player activates Tree's ability; opponent destroys the Tree in response; when the ability resolves neither life total nor toughness should change.

### tree_bounced_in_response_no_exchange
Scenario: Player activates Tree's ability; opponent bounces the Tree to hand in response; when the ability resolves neither life total nor toughness should change.

