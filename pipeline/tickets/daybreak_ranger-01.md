---
id: daybreak_ranger-01
status: new
card: Daybreak Ranger
card_file: mtg-engine/src/cards/isd/daybreak_ranger.rs
created: 2026-04-14T21:22:02Z
audit_run_id: 2026-04-14-daybreak_ranger-audit
audit_model: opus
audit_tokens: 12078
audit_duration: 368
---

## Audit Finding

**Oracle text:**
> {T}: This creature deals 2 damage to target creature with flying.

**Code:**
> `if let Some(obj) = state.get_object_mut(*target_id) { if obj.zone == Zone::Battlefield { obj.damage_marked += 2; obj.damaged_by.push(object_id); } }` — daybreak_ranger.rs:130–134

**Description:**
The front face's activated ability deals damage by directly mutating `obj.damage_marked` instead of using the central damage handler (`PendingEffect::DealDamage` resolved through `apply_pending_effect` at engine.rs:3424). The central handler checks protection from source (engine.rs:3449), damage prevention/replacement effects (engine.rs:3426–3447), and handles planeswalker loyalty removal. The inline path bypasses all of these. Additionally, if Daybreak Ranger were granted lifelink, the inline path would not grant life (the central handler doesn't either for creature targets, but the fight helper `deal_fight_damage` at combat.rs:200–211 does handle lifelink). Per CR 120.3, damage must go through the full results pipeline; inlining it silently drops protections and replacement effects.

**Engine path:**
- daybreak_ranger.rs:130–134 (inline damage)
- engine.rs:3424–3478 (central handler that should be used)

**Required check:** 8e

**Affected cards:**
- Daybreak Ranger (front face activated ability)
- Any other card that inlines `damage_marked +=` instead of using `PendingEffect::DealDamage`

