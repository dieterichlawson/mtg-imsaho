---
id: blasphemous_act-01
status: new
card: Blasphemous Act
card_file: mtg-engine/src/cards/isd/blasphemous_act.rs
created: 2026-04-15T03:42:48Z
audit_run_id: 2026-04-14-blasphemous_act-audit
audit_model: opus
audit_tokens: 8997
audit_duration: 222
---

## Audit Finding

**Oracle text:**
> Blasphemous Act deals 13 damage to each creature.

**Code:**
> `obj.damage_marked += 13; obj.damaged_by.push(object_id);` — blasphemous_act.rs:54-55

**Description:**
The `on_resolve` method applies damage by directly writing to `obj.damage_marked` instead of using the central `apply_pending_effect` with `PendingEffect::DealDamage`. The central damage path (engine.rs:3424-3478) checks for damage-prevention replacement effects (`PreventDamageRemoveCounter`, used by Unbreathing Horde — engine.rs:3426-3448) and protection from source (`has_protection_from` — engine.rs:3449-3453) before applying damage. The inline path bypasses both checks, meaning: (1) a creature with protection from red illegally takes 13 damage from Blasphemous Act (CR 702.16d: protection prevents all damage from sources with the given quality), and (2) a creature with a damage-prevention replacement effect (e.g., Unbreathing Horde's "prevent damage, remove counter") has the replacement effect ignored (CR 614.1).

**Engine path:**
- mtg-engine/src/cards/isd/blasphemous_act.rs:52-62 (inline damage loop)
- mtg-engine/src/engine.rs:3424-3478 (central DealDamage handler with protection + replacement checks)

**Required check:** 8e

**Affected cards:**
- Blasphemous Act
- Any other card that inlines damage via `obj.damage_marked += N` instead of using PendingEffect::DealDamage

## Tests

### blasphemous_act_protection_from_red_prevents_damage
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place a creature with protection from red (e.g., via a continuous effect or a creature that naturally has it) and another creature without protection on the battlefield. Cast and resolve Blasphemous Act. Assert the protected creature has 0 damage marked, and the unprotected creature has 13 damage marked.

### blasphemous_act_damage_prevention_replacement_effect
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place an Unbreathing Horde (which has PreventDamageRemoveCounter) with +1/+1 counters on the battlefield alongside a vanilla creature. Cast and resolve Blasphemous Act. Assert that Unbreathing Horde lost a +1/+1 counter instead of taking damage (damage_marked remains 0), and the vanilla creature has 13 damage marked.

