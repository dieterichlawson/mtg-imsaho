---
id: into_the_maw_of_hell-01
status: new
card: Into the Maw of Hell
card_file: mtg-engine/src/cards/isd/into_the_maw_of_hell.rs
created: 2026-04-14T21:28:27Z
audit_run_id: 2026-04-14-into_the_maw_of_hell-audit
audit_model: opus
audit_tokens: 12222
audit_duration: 253
---

## Audit Finding

**Oracle text:**
> Into the Maw of Hell deals 13 damage to target creature.

**Code:**
> `obj.damage_marked += 13;` (into_the_maw_of_hell.rs:70)
> `obj.damaged_by.push(object_id);` (into_the_maw_of_hell.rs:71)
> Manual `NonCombatDamageDealt` event push (into_the_maw_of_hell.rs:73-77)

**Description:**
The card inlines damage by directly writing `damage_marked += 13` instead of using the central `PendingEffect::DealDamage` path in `apply_pending_effect` (engine.rs:3424). The central handler (engine.rs:3426-3478) checks for damage prevention replacement effects (e.g., Unbreathing Horde's counter removal at line 3426-3448), protection from source (line 3449-3453), and planeswalker loyalty removal (line 3460-3466). By inlining, the card bypasses protection from source (CR 702.16 — a creature with protection from red should prevent the 13 damage) and all damage replacement/prevention effects (CR 614). The card does correctly emit a `NonCombatDamageDealt` event and track `damaged_by`, so triggers fire, but the damage itself is not processed through the rules-correct path. Other Innistrad cards (Burning Vengeance, Rage Thrower, Pitchburn Devils, Curse of the Pierced Heart) correctly use `PendingEffect::DealDamage`.

**Engine path:**
- into_the_maw_of_hell.rs:67-81 (inline damage application)
- engine.rs:3424-3478 (correct central damage handler that is bypassed)

**Required check:** 8e

**Affected cards:**
- Into the Maw of Hell

