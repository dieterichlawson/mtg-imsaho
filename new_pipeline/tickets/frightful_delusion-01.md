---
id: frightful_delusion-01
status: new
card: Frightful Delusion
audit_run_id: 2026-04-19-frightful_delusion-audit
audit_model: sonnet
audit_tokens: 12774
audit_duration: 838
---

## Audit Finding

**Oracle text:**
> Counter target spell unless its controller pays {1}.

**Code:**
> let can_pay = state.get_player(controller).mana_pool.total() >= 1;

                    if can_pay {
                        // Opponent has mana -- ask them to choose.
                        ...
                        return; // Don't clean up yet
                    }

                    // Can't pay -- auto-counter.

**Description:**
When the target spell's controller has no mana currently floating in their mana pool (mana_pool.total() < 1), the code unconditionally auto-counters the spell without offering the player a choice. Per CR 608.2g, when a resolving effect gives a player the option to pay mana, that player may activate mana abilities (e.g., tap lands) before paying. A player who controls untapped Islands but has 0 floating mana should be offered the PayOrNot prompt and allowed to tap a land to generate {1}; instead, their spell is auto-countered and they are still forced to discard. The check should not gate whether the player is prompted — it should only gate whether PayDecision(true) can successfully deduct from the pool. The fix is to always present the PayOrNot choice and let the player decide, with the engine refusing to complete a PayDecision(true) action if the pool cannot actually cover the cost.

**Engine path:** mtg-engine/src/cards/isd/frightful_delusion.rs:49

## Tests

### auto_counter_when_controller_has_no_floating_mana_but_has_lands
Scenario: Target spell's controller has 0 floating mana but controls an untapped Island; Frightful Delusion resolves — engine should present the PayOrNot prompt, but instead auto-counters the spell without asking.

### player_offered_choice_when_controller_has_floating_mana
Scenario: Target spell's controller has {U} floating; Frightful Delusion resolves — engine should present the PayOrNot prompt (positive control confirming the if-branch works correctly).

