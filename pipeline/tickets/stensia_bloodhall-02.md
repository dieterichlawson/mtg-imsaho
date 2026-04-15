---
id: stensia_bloodhall-02
status: closed-duplicate
card: Stensia Bloodhall
card_file: mtg-engine/src/cards/isd/stensia_bloodhall.rs
created: 2026-04-15T03:48:41Z
audit_run_id: 2026-04-14-stensia_bloodhall-audit
audit_model: opus
audit_tokens: 8416
audit_duration: 225
duplicate_of: merged-ability-targets-protection-02
---

## Audit Finding

**Oracle text:**
> {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.

**Code:**
> `generate_ability_targets` for `PlayerOrPlaneswalker` (engine.rs:2033-2051) calls `can_be_targeted(state, obj.id, controller, registry)` at line 2044, which delegates to `can_be_targeted_by(state, target_id, caster, None, registry)` at engine.rs:1446, passing `None` for `source_id`.

**Description:**
When enumerating valid targets for activated abilities, the engine calls `can_be_targeted` instead of `can_be_targeted_by` with the source permanent's ID. The `can_be_targeted_by` function checks protection from the source at engine.rs:1461-1465, but only when `source_id` is `Some`. By passing `None`, the protection check is skipped entirely. This means a planeswalker with protection from colorless (or protection from lands, etc.) can be illegally targeted by Stensia Bloodhall's activated ability. Per CR 702.16c, protection prevents targeting by sources with the specified quality. This is an engine-wide issue affecting all cards with targeted activated abilities.

**Engine path:**
- engine.rs:2044 (`can_be_targeted` call in `PlayerOrPlaneswalker` branch)
- engine.rs:1445-1447 (`can_be_targeted` wrapper passing `None`)
- engine.rs:1452-1467 (`can_be_targeted_by` with protection check)

**Required check:** 8f

**Affected cards:**
- Stensia Bloodhall
- All cards with targeted activated abilities (e.g., Daybreak Ranger, Skirsdag Cultist, any tap-to-damage or tap-to-exile ability)

## Tests

### bloodhall_cannot_target_protected_planeswalker
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Stensia Bloodhall on the battlefield controlled by P0. Place a planeswalker controlled by P1 that has protection from colorless. Give P0 sufficient mana ({3}{B}{R}). Call `legal_actions` and assert that no `ActivateAbility` action for Bloodhall targets the protected planeswalker. The planeswalker should not appear in the target list.
