---
id: snapcaster_mage-03
status: new
card: Snapcaster Mage
audit_run_id: 2026-04-19-snapcaster_mage-audit
audit_model: sonnet
audit_tokens: 34984
audit_duration: 719
---

## Audit Finding

**Oracle text:**
> If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way.

**Code:**
> let cost = registry.card_data(obj.card_id)
    .and_then(|d| d.cost.clone())
    .unwrap_or_else(ManaCost::free);

**Description:**
The `on_enter_battlefield` handler converts `cost: None` (no mana cost) to `ManaCost::free()` via `unwrap_or_else`. When Snapcaster Mage targets an instant or sorcery with no mana cost, this grants it a zero-mana flashback cost, making it castable for free. Per the ruling, a card with no mana cost that gains flashback has no flashback cost and cannot be cast via flashback at all. The fix is to propagate `None` — if `d.cost` is `None`, skip pushing the `GrantFlashback` effect entirely (or guard the grant) rather than substituting a free cost. The identical bug exists in the `PendingEffect::GrantFlashback` handler at engine.rs:3946 (`unwrap_or(ManaCost::free())`) and in Past in Flames at past_in_flames.rs:53 (`unwrap_or(ManaCost::free())`), which use the same fallback pattern.

**Engine path:** mtg-engine/src/cards/isd/snapcaster_mage.rs:63

**Required check:** 8j

**Affected cards:**
- Past in Flames

## Tests

### snapcaster_does_not_grant_usable_flashback_to_zero_mana_cost_card
Scenario: An instant or sorcery with no mana cost (cost: None) is in the controller's graveyard; after Snapcaster's ETB trigger resolves, that card should not appear as a castable flashback action, but currently it can be cast for free.

