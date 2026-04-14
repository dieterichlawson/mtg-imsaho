---
id: devils_play-01
status: new
card: Devil's Play
card_file: mtg-engine/src/cards/isd/devils_play.rs
created: 2026-04-14T21:24:56Z
audit_run_id: 2026-04-14-devils_play-audit
audit_model: opus
audit_tokens: 12112
audit_duration: 301
---

## Audit Finding

**Oracle text:**
> Devil's Play deals X damage to any target.

**Code:**
> `helpers.rs:55`: `obj.damage_marked += amount;` — applied unconditionally to all `Target::Object` targets regardless of whether the target is a creature or planeswalker, and without checking protection or damage prevention.

**Description:**
Devil's Play uses `resolve_damage` (helpers.rs:49-83) which inlines damage dealing instead of routing through the central `PendingEffect::DealDamage` handler (engine.rs:3424-3479). The central handler (1) checks `PreventDamageRemoveCounter` replacement effects, (2) checks protection from the damage source (CR 702.16), and (3) distinguishes creatures from planeswalkers — marking damage on creatures but removing loyalty counters on planeswalkers (CR 120.3c: "Damage dealt to a planeswalker removes that many loyalty counters from it"). The inline `resolve_damage` skips all three checks. Concretely: if Devil's Play targets a planeswalker, it marks `damage_marked` on the planeswalker object instead of removing loyalty counters, which is incorrect per CR 120.3c and CR 306.8. If it targets a creature with protection from red, the damage is dealt anyway instead of being prevented.

**Engine path:**
- `mtg-engine/src/cards/isd/devils_play.rs:52` — calls `resolve_damage`
- `mtg-engine/src/cards/helpers.rs:55` — `obj.damage_marked += amount` (skips protection, skips planeswalker loyalty removal)
- `mtg-engine/src/engine.rs:3424-3479` — central `DealDamage` handler that correctly handles protection (line 3449) and planeswalker loyalty (lines 3456-3466)

**Required check:** 8e

**Affected cards:**
- Devil's Play
- All other cards using `resolve_damage`: Lightning Bolt, Brimstone Volley, Geistflame, and any card calling `helpers::resolve_damage`

