---
id: grimgrin_corpse_born-01
status: fixed
card: Grimgrin, Corpse-Born
audit_run_id: 2026-04-19-grimgrin_corpse_born-audit
audit_model: sonnet
audit_tokens: 34446
audit_duration: 577
fixed_sha: 76d0ef84877d7dbd295f0f1fd8df00821e97f692
fixed_at: 2026-08-24T00:38:42Z
test_file: mtg-engine/tests/trigger_target_recheck.rs
fix_note: CR 608.2b re-check now runs is_valid_target too, matching resolve_spell
---

## Audit Finding

**Oracle text:**
> [2011-09-22] If the targeted creature is an illegal target by the time Grimgrin's last ability resolves, the entire ability doesn't resolve and none of its effects will occur. You won't put a +1/+1 counter on Grimgrin.

**Code:**
>         let any_legal = targets.iter().any(|t| {
            crate::stack::is_target_legal(state, t, &target_req, controller, registry)
        });
        if !any_legal {

**Description:**
The CR 608.2b re-check in `resolve_next_trigger` calls only `is_target_legal` (which checks zone, hexproof, and TargetFilter), but never calls `is_valid_target` (the card-specific restriction). Grimgrin's `is_valid_target` enforces the 'defending player controls' requirement by checking `obj.controller == defender`. If the targeted creature changes controller between trigger announcement and resolution — for example, Grimgrin's controller casts Act of Treason stealing the targeted creature — `is_target_legal` returns true (the creature is still on the battlefield without hexproof), but `is_valid_target` would return false (the creature is no longer controlled by the defending player). The trigger incorrectly resolves, destroying and adding a counter, when it should fizzle per the ruling. By contrast, `resolve_spell` in stack.rs correctly calls both `is_target_legal` AND `b.is_valid_target` at resolution time. Any card with a targeted triggered ability and a custom `is_valid_target` is affected by the same engine gap.

**Engine path:** mtg-engine/src/triggers.rs:1275

**Required check:** 8j

**Affected cards:**
- Reaper from the Abyss
- Morkrut Banshee

## Tests

### attack_trigger_fizzles_when_target_changes_controller
Scenario: Grimgrin attacks; attack trigger goes on stack with defending player's creature as target; in response, Grimgrin's controller steals that creature via Act of Treason; trigger should fizzle (target now illegal) but instead resolves and destroys the stolen creature

