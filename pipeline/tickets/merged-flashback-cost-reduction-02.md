---
id: merged-flashback-cost-reduction-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: army_of_the_damned-02, creeping_renaissance-02, devils_play-02, heartless_summoning-01, heartless_summoning-02, snapcaster_mage-03, unburial_rites-01, sever_the_bloodline-02, travel_preparations-01, merged-flashback-cost-reduction-01
---

# Alternative-cost paths bypass effective_spell_cost (CR 601.2f)

## Description
Per CR 601.2f, the total cost of a spell starts with the mana cost or alternative cost (such as flashback), then adds cost increases and subtracts cost reductions. The engine's `effective_spell_cost` function (engine.rs:261-305) applies `ReduceCost` continuous effects but is only called on the normal-cast branch. The flashback cast path (engine.rs:2219-2227) and the `legal_actions` flashback affordability check (engine.rs:1263-1291) both use the raw flashback cost, bypassing active cost reducers. This produces both incorrect affordability checks and incorrect payment amounts for all flashback spells when cost reduction is active.

## Engine path
- engine.rs:2219-2227 (submit_action flashback branch — raw cost)
- engine.rs:2229-2230 (submit_action normal branch — correctly reduced)
- engine.rs:1263-1291 (legal_actions flashback affordability — raw cost)
- engine.rs:261-305 (effective_spell_cost — the function that should be called)

## Tests

### test_army_of_the_damned_flashback_cost_reduced
Source ticket: army_of_the_damned-02
Implementation: (not yet written)
Scenario: Control a permanent with `ReduceCost { reduction: 1, filter: SpellFilter::All }`. Cast Army of the Damned from graveyard via flashback. Verify the paid cost is 1 less than the raw flashback cost.

### test_creeping_renaissance_flashback_cost_reduced
Source ticket: creeping_renaissance-02
Implementation: (not yet written)
Scenario: Control a permanent reducing sorcery costs by {1}. Cast Creeping Renaissance via flashback. Verify the reduction is applied.

### test_devils_play_flashback_cost_reduced
Source ticket: devils_play-02
Implementation: (not yet written)
Scenario: Control a permanent reducing instant/sorcery costs by {1}. Cast Devil's Play from graveyard via flashback with X=2. Verify the mana paid is 1 less than the raw {X}{R}{R} flashback cost.

### test_heartless_summoning_applies_to_granted_flashback
Source ticket: heartless_summoning-01
Implementation: (not yet written)
Scenario: Control Heartless Summoning (reduces creature spell costs by {2}). Grant flashback to a creature spell in graveyard (e.g., via future Snapcaster-like effect with a creature target). Verify the flashback cast benefits from the {2} reduction.

### test_heartless_summoning_reduces_skaab_ruinator_graveyard_cast
Source ticket: heartless_summoning-02
Implementation: (not yet written)
Scenario: Control Heartless Summoning. Have Skaab Ruinator in graveyard. Verify `legal_actions` lists Skaab Ruinator as castable at reduced cost and `submit_action` pays the reduced cost when selected.

### test_snapcaster_granted_flashback_cost_reduced
Source ticket: snapcaster_mage-03
Implementation: (not yet written)
Scenario: Control Heartless Summoning. ETB Snapcaster targeting a creature spell in graveyard. Verify casting that spell via granted flashback pays the reduced cost.

### test_unburial_rites_flashback_cost_reduced
Source ticket: unburial_rites-01
Implementation: (not yet written)
Scenario: Control a permanent with `ReduceCost { reduction: 1, filter: SpellFilter::All }`. Cast Unburial Rites via flashback. Verify the reduction applies.

### test_sever_the_bloodline_flashback_cost_reduced
Source ticket: sever_the_bloodline-02
Implementation: (not yet written)
Scenario: Control a permanent with `ReduceCost { reduction: 1, filter: SpellFilter::All }`. Cast Sever the Bloodline from graveyard via flashback. Verify the paid cost is {4}{B}{B} (reduced from {5}{B}{B}).

### test_travel_preparations_flashback_cost_reduced
Source ticket: travel_preparations-01
Implementation: (not yet written)
Scenario: Control a permanent reducing sorcery costs by {1}. Cast Travel Preparations via flashback. Verify the flashback cost is reduced from {1}{W} to {W}.

## Also closes

- merged-flashback-cost-reduction-01

