---
id: snapcaster_mage-01
status: new
card: Snapcaster Mage
audit_run_id: 2026-04-19-snapcaster_mage-audit
audit_model: sonnet
audit_tokens: 34984
audit_duration: 719
---

## Audit Finding

**Oracle text:**
> target instant or sorcery card in your graveyard gains flashback until end of turn

**Code:**
> let already_granted = state.until_end_of_turn.iter().any(|e| matches!(e,
    crate::state::TemporaryEffect::GrantFlashback { target, .. } if *target == *id));
!already_granted

**Description:**
The `is_valid_target` implementation adds an `!already_granted` guard that blocks targeting any instant or sorcery card that already has a `GrantFlashback` effect in `until_end_of_turn`. No such restriction appears in the oracle text — "target instant or sorcery card in your graveyard" has no clause excluding cards that already have flashback. A second Snapcaster Mage entering after the first one's trigger has resolved will find no legal targets if the only instant or sorcery in the graveyard already received a `GrantFlashback` grant, causing the second trigger to be removed from the stack per CR 603.3c when it should still be able to target that card (redundantly granting flashback, as explicitly allowed by the ruling "If a card has multiple instances of flashback, you may choose any of its flashback costs to pay"). The guard should be removed entirely.

**Engine path:** mtg-engine/src/cards/isd/snapcaster_mage.rs:53

**Required check:** 6

## Tests

### second_snapcaster_trigger_fizzles_with_only_one_valid_target
Scenario: One instant in graveyard; first Snapcaster's ETB trigger resolves granting it flashback; second Snapcaster enters — its ETB trigger should present the instant as a valid target but currently finds no legal targets and is removed from the stack.

### snapcaster_can_target_card_with_existing_dynamic_flashback
Scenario: Instant in graveyard already has GrantFlashback in until_end_of_turn; Snapcaster ETB trigger resolves and the controller should be able to target that instant, redundantly granting flashback at its mana cost.

