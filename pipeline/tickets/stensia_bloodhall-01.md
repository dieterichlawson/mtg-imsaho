---
id: stensia_bloodhall-01
status: closed-duplicate
card: Stensia Bloodhall
card_file: mtg-engine/src/cards/isd/stensia_bloodhall.rs
created: 2026-04-15T03:48:41Z
audit_run_id: 2026-04-14-stensia_bloodhall-audit
audit_model: opus
audit_tokens: 8416
audit_duration: 225
duplicate_of: merged-inline-damage-02
---

## Audit Finding

**Oracle text:**
> {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.

**Code:**
> `on_activate_ability` (stensia_bloodhall.rs:65-103) directly manipulates player life (`state.get_player_mut(*player_id).life = new_life` at line 70) and planeswalker loyalty counters (`*loyalty = loyalty.saturating_sub(2)` at line 89) instead of using the central `apply_pending_effect` with `PendingEffect::DealDamage`.

**Description:**
The damage is inlined rather than routed through the central damage handler (`apply_pending_effect` in engine.rs:3418). The central handler checks protection from the source (engine.rs:3449, CR 702.16), damage prevention/replacement effects like `PreventDamageRemoveCounter` (engine.rs:3426-3448, CR 614), and tracks `damaged_by` for planeswalkers (engine.rs:3470). The inline code bypasses all of these. For the player path, it also bypasses any player-targeted damage replacement. For the planeswalker path, it additionally fails to push to `obj.damaged_by`, breaking any downstream logic that depends on damage-source tracking. If the land somehow gained lifelink (e.g., via an Aura), the inline path would not grant life (CR 702.15).

**Engine path:**
- stensia_bloodhall.rs:65-103 (inline damage in `on_activate_ability`)
- engine.rs:3418-3491 (central `apply_pending_effect` that should be used)

**Required check:** 8e

**Affected cards:**
- Stensia Bloodhall
- Any other card that inlines damage in `on_activate_ability` or `on_resolve`

## Tests

### bloodhall_damage_respects_protection
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Stensia Bloodhall on the battlefield controlled by P0. Place a planeswalker controlled by P1 with protection from colorless (or use a continuous effect granting it). Activate Bloodhall's ability targeting the planeswalker. Assert that the planeswalker's loyalty counters are unchanged because protection prevents the damage.

### bloodhall_damage_respects_prevention
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Stensia Bloodhall on the battlefield controlled by P0. Place a creature with `PreventDamageRemoveCounter` (e.g., Unbreathing Horde with +1/+1 counters) controlled by P1. Since Bloodhall targets "player or planeswalker" and not creatures, instead set up a player-targeted damage prevention effect if one exists, or verify via the planeswalker path: place a planeswalker controlled by P1 with a damage prevention shield. Activate Bloodhall targeting the planeswalker. Assert that damage is prevented and the prevention effect is consumed, not that loyalty is directly reduced.
