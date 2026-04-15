---
id: harvest_pyre-01
status: closed-duplicate
card: Harvest Pyre
card_file: mtg-engine/src/cards/isd/harvest_pyre.rs
created: 2026-04-15T03:41:13Z
audit_run_id: 2026-04-14-harvest_pyre-audit
audit_model: opus
audit_tokens: 5832
audit_duration: 127
duplicate_of: merged-inline-damage-02
---

## Audit Finding

**Oracle text:**
> Harvest Pyre deals X damage to target creature.

**Code:**
> `obj.damage_marked += count;` at harvest_pyre.rs:48, followed by manual `obj.damaged_by.push(object_id)` at line 49 and manual `NonCombatDamageDealt` event push at lines 53-57.

**Description:**
Harvest Pyre applies damage by directly writing `obj.damage_marked += count` instead of using the central `PendingEffect::DealDamage` path in `apply_pending_effect` (engine.rs:3424-3478). The central handler checks for damage prevention/replacement effects (`PreventDamageRemoveCounter` at engine.rs:3426-3448, used by Unbreathing Horde), protection from source (`has_protection_from` at engine.rs:3449-3453), and planeswalker loyalty counter removal (engine.rs:3460-3466). The inline path bypasses all of these. Concretely: a creature with a damage-prevention replacement effect (e.g., Unbreathing Horde with +1/+1 counters) or protection from red would still take the full damage from Harvest Pyre, violating CR 702.16e (protection prevents damage) and CR 614.1 (replacement effects).

**Engine path:**
- mtg-engine/src/cards/isd/harvest_pyre.rs:46-57
- mtg-engine/src/engine.rs:3424-3478 (correct path that is not used)

**Required check:** 8e

**Affected cards:**
- Harvest Pyre
- Any other card using inline `damage_marked +=` instead of `PendingEffect::DealDamage`

## Tests

### harvest_pyre_inline_damage_bypasses_protection
Source ticket: (new)
Implementation: (not yet written)
Scenario: Set up a battlefield with a creature that has protection from red (e.g., via a continuous effect or inherent ability). Cast Harvest Pyre exiling cards from graveyard, targeting that creature. Assert the creature takes 0 damage (protection prevents all damage from red sources). Currently fails because inline damage skips the protection check.

### harvest_pyre_inline_damage_bypasses_prevention
Source ticket: (new)
Implementation: (not yet written)
Scenario: Set up a battlefield with Unbreathing Horde (which has PreventDamageRemoveCounter — damage is prevented and a +1/+1 counter is removed instead). Cast Harvest Pyre targeting Unbreathing Horde. Assert that damage is prevented and a +1/+1 counter is removed. Currently fails because inline damage skips the replacement effect check.
