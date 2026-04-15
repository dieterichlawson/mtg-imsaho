---
id: balefire_dragon-01
status: deduped
card: Balefire Dragon
card_file: mtg-engine/src/cards/isd/balefire_dragon.rs
created: 2026-04-14T20:36:05Z
audit_run_id: 2026-04-14-balefire_dragon-audit
audit_model: opus
audit_tokens: 6049
audit_duration: 124
deduped_into: merged-inline-damage-01
---

## Audit Finding

**Oracle text:**
> Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.

**Code:**
> `balefire_dragon.rs:54-56`:
> ```rust
> if let Some(obj) = state.get_object_mut(creature_id) {
>     obj.damage_marked += amount;
>     obj.damaged_by.push(self_id);
> }
> ```

**Description:**
The triggered ability deals damage by directly writing `obj.damage_marked += amount` instead of routing through the central damage handler (`apply_pending_effect` with `PendingEffect::DealDamage` at engine.rs:3424). The central handler (engine.rs:3424-3478) checks for: (1) damage prevention/replacement effects such as Unbreathing Horde's counter-removal prevention (engine.rs:3426-3447), (2) protection from source via `has_protection_from` (engine.rs:3449-3453), and (3) planeswalker loyalty counter removal for planeswalker targets (engine.rs:3460-3466). By inlining the damage, Balefire Dragon's triggered ability bypasses all three of these. For example, a creature with protection from red (or protection from Dragons) would incorrectly receive damage from the ability, violating CR 702.16e ("If a source would deal damage to a permanent or player protected from it, the damage is prevented"). The card does correctly emit `NonCombatDamageDealt` events and push to `damaged_by`, so triggers and deathtouch tracking work, but the damage itself is not subject to any prevention or replacement layer.

**Engine path:**
- balefire_dragon.rs:54-56 (inline damage write)
- engine.rs:3424-3478 (central damage handler that should be used)
- engine.rs:3449-3453 (protection check bypassed)
- engine.rs:3426-3447 (prevention/replacement check bypassed)
- engine.rs:3460-3466 (planeswalker loyalty removal bypassed)

**Required check:** 8e

**Affected cards:**
- Balefire Dragon
- Any other card using inline `obj.damage_marked += amount` for non-combat damage in a triggered/activated ability
