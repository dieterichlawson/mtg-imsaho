---
id: grimoire_of_the_dead-02
status: new
card: Grimoire of the Dead
audit_run_id: 2026-04-19-grimoire_of_the_dead-audit
audit_model: sonnet
audit_tokens: 22452
audit_duration: 439
---

## Audit Finding

**Oracle text:**
> {T}, Remove three study counters from Grimoire of the Dead and sacrifice it:

**Code:**
> // Note: The engine already handled tapping and sacrificing as part of the cost.
                // The study counters were already checked in activated_abilities().
                // (Since the Grimoire is now in the graveyard, counter removal is moot.)

**Description:**
The activation cost includes 'Remove three study counters from Grimoire of the Dead' as a discrete cost action. The engine's ActivatedAbilityDef has no field for counter-removal costs, so on_activate_ability is relied upon to handle this. However, the comment explicitly documents that counter removal is skipped: when SacrificeCost::SacrificeThis is processed before on_activate_ability is called, state.move_object moves the Grimoire to the graveyard and its cleanup block calls obj.counters.clear(), clearing all counters in one step. The 'remove three counters' cost action never runs. This has two concrete consequences: (1) any replacement effect that applies to counter removal from Grimoire never fires, and (2) if the player has activated ability 0 more than three times (accumulating e.g. four or five study counters), the gating condition in activated_abilities() still offers ability 1 (study_counters >= 3 is true), but when it resolves all counters are cleared rather than exactly three being removed. The oracle requires exactly three counters to be removed as a cost, with the remainder staying on the permanent — but since the permanent is simultaneously sacrificed the excess counters vanish without ever having been 'removed' per the rules.

**Engine path:** mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:131

**Required check:** 8c

## Tests

### only_three_counters_removed_when_four_present
Scenario: Grimoire has four study counters (ability 0 activated four times). Ability 1 is activated. After resolution, verify that exactly three counters were consumed as the cost — the fourth counter should conceptually remain until the sacrifice clears it, and any counter-removal triggers should fire exactly three times.

### counter_removal_replacement_effect_applies
Scenario: With a replacement effect in play that prevents counter removal from artifacts, activating ability 1 should be prevented (costs cannot be paid), not silently bypass the replacement by clearing counters via sacrifice.

