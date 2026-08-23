---
id: past_in_flames-02
status: new
card: Past in Flames
audit_run_id: 2026-04-19-past_in_flames-audit
audit_model: sonnet
audit_tokens: 47972
audit_duration: 925
---

## Audit Finding

**Oracle text:**
> If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way.

**Code:**
> if is_instant_or_sorcery {
    Some((o.id, d.cost.clone().unwrap_or(ManaCost::free())))
}

**Description:**
For any instant or sorcery card whose `cost` field is `None` (no mana cost), the code falls back to `ManaCost::free()`, which is `ManaCost { symbols: vec![] }` — an empty-symbol cost that the engine treats as payable at zero mana. This would cause Past in Flames to grant such a card flashback with a {0} effective cost, making it castable for free from the graveyard. The ruling is unambiguous: a card with no mana cost that gains flashback has no flashback cost and cannot be cast via flashback. The correct fix is to skip cards with `d.cost.is_none()` entirely in the filter — do not add them to the GrantFlashback list. No currently implemented instant or sorcery in the engine has `cost: None`, so this bug is latent rather than immediately observable. The same `unwrap_or(ManaCost::free())` fallback also appears in the Snapcaster Mage ETB handler and the GrantFlashback PendingEffect handler in engine.rs:3946.

**Engine path:** mtg-engine/src/cards/isd/past_in_flames.rs:53

**Required check:** 8j

**Affected cards:**
- Snapcaster Mage

## Tests

### past_in_flames_no_mana_cost_card_not_granted_flashback
Scenario: An instant card with no mana cost (cost: None) is in the graveyard when Past in Flames resolves; it should not appear as a legal flashback cast action afterward

