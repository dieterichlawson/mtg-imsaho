---
id: demonmail_hauberk-01
status: new
card: Demonmail Hauberk
audit_run_id: 2026-04-19-demonmail_hauberk-audit
audit_model: sonnet
audit_tokens: 28278
audit_duration: 508
---

## Audit Finding

**Oracle text:**
> Equip—Sacrifice a creature.

**Code:**
> .filter(|o| already_attached != Some(o.id))

**Description:**
In `generate_ability_targets` (engine.rs:2022), when the source is an Equipment with an active attachment, the code computes `already_attached = source.attached_to` and then filters it out of the valid target list. Per CR 702.6a, the equip ability reads "Pay [cost]: Attach this permanent to target creature you control" — there is no rule that prevents targeting the creature the equipment is already attached to. This blocks the legal action of sacrificing a creature other than the equipped one while keeping the equipment on its current host: if Demonmail Hauberk is attached to creature A and the controller also controls creature B, the action (sacrifice=B, target=A) is never offered, even though it is a legal play. The blocked action has real strategic value whenever a player wants to exploit the sacrifice as a trigger source (e.g., triggering "whenever a creature dies" abilities) while preserving the equipment's current attachment. The only action offered — (sacrifice=A, target=B) — is the scenario the ruling describes and is correctly implemented; the bug is in the reverse case. All Equipment cards that use `TargetRequirement::CreatureWithFilter` share this code path and are equally affected.

**Engine path:** mtg-engine/src/engine.rs:2022

**Required check:** 8c

**Affected cards:**
- Sharpened Pitchfork
- Inquisitor's Flail
- Blazing Torch
- Wooden Stake
- Mask of Avacyn
- Cobbled Wings
- Trepanation Blade
- Butcher's Cleaver
- Silver-Inlaid Dagger
- Runechanter's Pike

## Tests

### equip_to_already_attached_creature_is_legal
Scenario: Demonmail Hauberk is attached to creature A; controller also controls creature B; the action (sacrifice=B, target=A) — sacrificing B while re-equipping to A — should appear in legal actions but does not.

