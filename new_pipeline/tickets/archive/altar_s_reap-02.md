---
id: altar_s_reap-02
status: could_not_confirm
card: Altar's Reap
audit_run_id: 2026-04-18-altar_s_reap-audit
audit_model: sonnet
audit_tokens: 10071
audit_duration: 184
---

## Audit Finding

**Oracle text:**
> You must sacrifice exactly one creature to cast this spell; you cannot cast it without sacrificing a creature, and you cannot sacrifice additional creatures.

**Code:**
> Some(AdditionalCost::SacrificeCreature) => {
    let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
        .iter()
        .filter(|o| o.power.is_some())
        .map(|o| o.id)
        .collect();
    if creatures.is_empty() { continue; }
    creatures
}

**Description:**
The implementation correctly excludes Altar's Reap from `legal_actions` when the casting player controls no creatures, matching the ruling. However, no test verifies this exclusion: there is no scenario that asserts the spell is absent from `legal_actions` (or from `castable_spells`) when the player has no creatures on the battlefield. Per the audit procedure for check 8j, the behavior appears correct but the ruling's edge case is unexercised.

**Engine path:** mtg-engine/src/engine.rs:1056

**Required check:** 8j

## Tests

### altars_reap_not_castable_without_creatures
Scenario: With no creatures on the battlefield, verify that Altar's Reap does not appear in `legal_actions().castable_spells` even though the player has sufficient mana.

## Test Run Results

- **altars_reap_not_castable_without_creatures** — rejected
  - explanation: The engine already correctly excludes Altar's Reap from castable_spells when the casting player controls no creatures. At engine.rs:1062, the code does `if creatures.is_empty() { continue; }` which skips adding the spell to the castable list. A test asserting this exclusion was written and run — it passed immediately against the current code, confirming the behavior is correct and no bug exists.

