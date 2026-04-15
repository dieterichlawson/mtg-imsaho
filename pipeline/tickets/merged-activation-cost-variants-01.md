---
id: merged-activation-cost-variants-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: back_from_the_brink-03, moorland_haunt-02, grimoire_of_the_dead-01, grimoire_of_the_dead-04, mikaeus_the_lunarch-02
---

# ActivatedAbilityDef lacks support for non-mana cost variants (CR 602.2b)

## Description
Per CR 602.2b, all of an activated ability's costs — mana, tap/untap, sacrifice, discard, exile-from-graveyard, counter-removal — are paid atomically at activation (rule 602.2f), before the ability goes on the stack. The engine's `ActivatedAbilityDef` supports mana costs, tap/untap costs, and sacrifice costs, but has no fields for discard costs, exile-from-graveyard costs, or counter-removal costs. Cards requiring those must defer the cost to `on_activate_ability`, interleaving cost payment with effect execution. This is masked today by the engine's no-stack activation (see merged-activated-no-stack-01) but leaves the game in a "half-activated" intermediate state and will break if Stifle-like effects are added.

## Engine path
- cards/mod.rs (ActivatedAbilityDef struct — no exile/counter/discard fields)
- engine.rs:2645-2653 (mana + tap paid)
- engine.rs:2660-2670 (sacrifice paid)
- engine.rs:2717-2719 (on_activate_ability — where non-mana costs are currently handled)

## Tests

### test_back_from_the_brink_exile_is_cost
Source ticket: back_from_the_brink-03
Implementation: (not yet written)
Scenario: Activate Back from the Brink choosing a creature in graveyard. Verify the exile happens before any effect is pushed to the stack / resolved (i.e., the creature is already in exile before the token creation resolves). If a hypothetical Stifle-like effect counters the ability, the exile must still stand.

### test_moorland_haunt_exile_is_cost
Source ticket: moorland_haunt-02
Implementation: (not yet written)
Scenario: Activate Moorland Haunt with multiple creatures in graveyard. Verify the exile choice is made during cost payment (not after tap and mana are paid), and the exile is atomic with the rest of the cost.

### test_grimoire_of_the_dead_counter_removal_is_cost
Source ticket: grimoire_of_the_dead-01
Implementation: (not yet written)
Scenario: Grimoire with 5 study counters. Activate the sacrifice ability. Verify exactly 3 counters are removed as a cost (not 5 via cleanup), and a counter-removed event is emitted.

### test_grimoire_of_the_dead_discard_is_cost
Source ticket: grimoire_of_the_dead-04
Implementation: (not yet written)
Scenario: Activate Grimoire's study-counter ability with multiple cards in hand. Verify the discard choice is made during cost payment, not after {1} and {T} have been paid.

### test_mikaeus_the_lunarch_counter_removal_is_cost
Source ticket: mikaeus_the_lunarch-02
Implementation: (not yet written)
Scenario: Mikaeus with +1/+1 counters. Activate the lifelink-granting ability. Verify the counter removal is a cost paid at activation (emitting an event), not part of the effect.

