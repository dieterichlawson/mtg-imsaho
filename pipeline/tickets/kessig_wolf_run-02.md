---
id: kessig_wolf_run-02
status: deduped
card: Kessig Wolf Run
card_file: mtg-engine/src/cards/isd/kessig_wolf_run.rs
created: 2026-04-14T21:29:34Z
audit_run_id: 2026-04-14-kessig_wolf_run-audit
audit_model: opus
audit_tokens: 14580
audit_duration: 309
deduped_into: merged-ability-targets-protection-01
---

## Audit Finding

**Oracle text:**
> {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.

**Code:**
> `fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool { can_be_targeted_by(state, target_id, caster, None, registry) }` — engine.rs:1445-1447
> Called from `generate_ability_targets` at engine.rs:2006: `.filter(|o| can_be_targeted(state, o.id, controller, registry))`

**Description:**
When enumerating valid targets for Kessig Wolf Run's activated ability, `generate_ability_targets` calls `can_be_targeted` which internally invokes `can_be_targeted_by` with `source_id: None`. The `can_be_targeted_by` function (engine.rs:1452-1465) only checks protection from a source when `source_id` is `Some`. With `None`, the protection check is skipped entirely. A creature with protection from red, protection from green, or protection from lands can be illegally targeted by this ability. Per CR 702.16c, a permanent with protection from [quality] cannot be targeted by sources with that quality. The spell-targeting path (`valid_targets_for_req`) correctly passes `Some(spell_id)`, but the activated ability path does not.

**Engine path:**
- engine.rs:1445-1447 (can_be_targeted passes None)
- engine.rs:1452-1465 (can_be_targeted_by skips protection when source_id is None)
- engine.rs:2003-2009 (generate_ability_targets for Creature)

**Required check:** 8f

**Affected cards:**
- Kessig Wolf Run
- Every card with a targeted activated ability (Daybreak Ranger, Olivia Voldaren, Grimgrin, etc.)
