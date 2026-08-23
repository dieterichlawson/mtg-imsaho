---
id: ghost_quarter-01
status: new
card: Ghost Quarter
audit_run_id: 2026-04-19-ghost_quarter-audit
audit_model: sonnet
audit_tokens: 25466
audit_duration: 1899
---

## Audit Finding

**Oracle text:**
> {T}, Sacrifice this land: Destroy target land.

**Code:**
> TargetFilter::HasCardType(types) => {
    types.iter().any(|t| obj.card_types.contains(t))
}

**Description:**
Ghost Quarter's activated ability uses `TargetFilter::HasCardType(vec![CardType::Land])` to enumerate valid targets. This filter resolves through `matches_target_filter` in engine.rs (line 2103), which checks only `obj.card_types` with no registry fallback. Per the existing insight, `create_object` initialises `card_types: Vec::new()` for every game object, and the default permanent-entry path (`move_object`) never sets `card_types` on non-token objects. Tokens are the exception: `create_token_with_subtypes` explicitly sets `card_types`. The result is that `obj.card_types.contains(&CardType::Land)` returns `false` for every non-token land on the battlefield, giving Ghost Quarter's sacrifice ability zero valid targets in all normal game positions. Compare the `HasSubtype` branch in the same function (line 2108), which correctly falls back to `registry.card_data(obj.card_id)` when the object's field is empty. The same root defect affects five other cards that use `HasCardType` for spell or ability targeting (see `affected_cards`).

**Engine path:** mtg-engine/src/engine.rs:2103

**Required check:** 8c

**Affected cards:**
- Ancient Grudge
- Naturalize
- Silverchase Fox
- Maw of the Mire
- Into the Maw of Hell

## Tests

### ghost_quarter_ability_no_targets_for_non_token_land
Scenario: Ghost Quarter's activated ability is used with an opponent's Forest as the intended target; the legal-actions enumeration should offer that Forest as a valid target but currently produces an empty list because HasCardType([Land]) returns false for non-token lands.

