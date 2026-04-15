---
id: olivia_voldaren-01
status: closed-duplicate
card: Olivia Voldaren
card_file: mtg-engine/src/cards/isd/olivia_voldaren.rs
created: 2026-04-14T20:44:31Z
audit_run_id: 2026-04-14-olivia_voldaren-audit
audit_model: opus
audit_tokens: 17927
audit_duration: 323
duplicate_of: merged-inline-damage-01
---

## Audit Finding

**Oracle text:**
> {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.

**Code:**
> `olivia_voldaren.rs:104-116` — directly writes `obj.damage_marked += 1` and manually handles planeswalker loyalty removal, bypassing the central `apply_pending_effect(DealDamage)` path at `engine.rs:3418`.

**Description:**
The first activated ability inlines damage instead of routing through the central damage handler. The central handler (`apply_pending_effect` at engine.rs:3424) checks `has_protection_from` (engine.rs:3449) and `PreventDamageRemoveCounter` replacement effects (engine.rs:3426-3447) before applying damage. The inline code skips both checks. A creature with protection from black, protection from red, or a damage-prevention replacement effect would incorrectly take 1 damage, become a Vampire, and give Olivia a +1/+1 counter. Per CR 702.16e, protection prevents damage from sources with the given quality; per CR 614.1a, replacement effects replace events as they happen.

**Engine path:**
- `olivia_voldaren.rs:104-116` (inline damage)
- `engine.rs:3424-3478` (central handler with protection/prevention)

**Required check:** 8e

**Affected cards:**
- Olivia Voldaren
