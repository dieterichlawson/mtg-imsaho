---
id: olivia_voldaren-03
status: new
card: Olivia Voldaren
audit_run_id: 2026-04-19-olivia_voldaren-audit
audit_model: sonnet
audit_tokens: 22278
audit_duration: 724
---

## Audit Finding

**Oracle text:**
> Gain control of target Vampire for as long as you control Olivia Voldaren.

**Code:**
> let is_vampire = state.get_object(*target_id)
    .is_some_and(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string()));
if is_vampire {

**Description:**
The resolution guard for ability 1 checks only obj.subtypes to determine whether the target is a Vampire. Non-token permanent cards enter the battlefield with obj.subtypes initialised as Vec::new() (create_object in state.rs), and no code copies registry subtypes onto the object on ETB. For non-token Vampires (Bloodline Keeper, Markov Patrician, etc.), obj.subtypes is empty — the registry is the authoritative source of their creature type. The target-enumeration filter (TargetFilter::HasSubtype) correctly checks both obj.subtypes AND registry, so these creatures appear as valid targets. But when ability 1 resolves, the guard returns false for any registry Vampire with empty obj.subtypes, and the control steal silently does nothing. The player pays {3}{B}{B}, targets a Vampire, the ability 'resolves', and nothing happens. The canonical engine pattern (state.rs:1487–1491, used in check_condition) checks the registry when obj.subtypes is empty and treats obj.subtypes as authoritative only when it is non-empty; the guard here must follow the same pattern.

**Engine path:** mtg-engine/src/cards/isd/olivia_voldaren.rs:125

**Required check:** 8d

## Tests

### ability1_steals_registry_vampire
Scenario: Opponent controls Markov Patrician (a Vampire by card type, empty obj.subtypes); activate Olivia's {3}{B}{B}; verify you gain control of Markov Patrician.

### ability1_steals_olivia_converted_vampire
Scenario: Olivia uses ability 0 to convert a Grizzly Bears into a Vampire (obj.subtypes now contains Vampire); then use ability 1 to steal that Vampire; verify you gain control of it.

