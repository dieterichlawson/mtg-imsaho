---
id: merged-activated-no-stack-02
status: confirmed
card: multiple
created: 2026-04-15T04:40:54Z
kind: consolidated
source_tickets: back_from_the_brink-02, kessig_wolf_run-03, nephalia_drownyard-01, tree_of_redemption-03, full_moons_rise-01, mirror_mad_phantasm-01, skirsdag_high_priest-01, merged-activated-no-stack-01
confirmed_at: 2026-04-15T06:47:23Z
test_run_id: 2026-04-14-merged-activated-no-stack-02-test
test_model: opus
test_tokens: 9309
test_duration: 223
test_file: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs
tests_confirmed: 10
tests_total: 10
worktree: /Users/dlaw/mtg/.worktrees/fix-merged-activated-no-stack-02
---

# Activated abilities resolve immediately with no stack entry (CR 602.2a)

## Description
Per CR 602.2a, activating an activated ability places an instance of it on the stack; players receive priority and may respond before it resolves. The engine has no `StackEntry` variant for activated abilities. The `ActivateAbility` handler (engine.rs:2559-2725) pays costs and immediately calls `on_activate_ability`, producing the effect atomically with activation. Opponents cannot respond — no Stifle, no sacrifice-for-value before removal, no destruction of the source to invalidate conditional effects. This is an engine-wide architectural limitation affecting every non-mana activated ability.

## Engine path
- state.rs:10-15 (StackEntry enum — no Ability variant)
- engine.rs:2559-2725 (ActivateAbility handler — atomic activation + effect)
- engine.rs:2717-2719 (immediate dispatch to on_activate_ability)
- engine.rs:3199-3214 (ChooseXFunding — X-cost path also resolves immediately)

## Tests

### test_back_from_the_brink_can_be_responded_to
Source ticket: back_from_the_brink-02
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::test_back_from_the_brink_can_be_responded_to
Scenario: Activate Back from the Brink's exile-and-create-token ability. Verify the ability is on the stack (observable via stack inspection) and opponents receive priority before it resolves.

### test_kessig_wolf_run_pump_can_be_responded_to
Source ticket: kessig_wolf_run-03
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::test_kessig_wolf_run_pump_can_be_responded_to
Scenario: Activate Kessig Wolf Run's pump with X=3. Verify the ability is on the stack before the pump applies; opponent can respond (e.g., remove the target).

### test_nephalia_drownyard_mill_can_be_responded_to
Source ticket: nephalia_drownyard-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::test_nephalia_drownyard_mill_can_be_responded_to
Scenario: Activate Nephalia Drownyard's mill ability. Verify the ability goes on the stack and the target player receives priority to respond.

### test_tree_of_redemption_exchange_can_be_responded_to
Source ticket: tree_of_redemption-03
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::test_tree_of_redemption_exchange_can_be_responded_to
Scenario: Activate Tree of Redemption's exchange ability. Verify the ability is on the stack; opponents can destroy Tree in response, causing the exchange to fail (per Scryfall ruling).

### full_moons_rise_regenerate_can_be_responded_to
Source ticket: full_moons_rise-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::full_moons_rise_regenerate_can_be_responded_to
Scenario: Player A controls Full Moon's Rise and a Werewolf creature. Player A activates the sacrifice-and-regenerate ability. Verify the ability goes on the stack, Player B receives priority to respond (e.g., by exiling the Werewolf), and regeneration shields are not applied until the ability resolves.

### mirror_mad_phantasm_opponent_can_respond
Source ticket: mirror_mad_phantasm-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::mirror_mad_phantasm_opponent_can_respond
Scenario: Player A controls Mirror-Mad Phantasm. Player A activates the ability (pays {1}{U}). Verify the ability goes on the stack and Player B receives priority. Player B can counter the ability (e.g., Stifle) or exile the Phantasm before it resolves.

### mirror_mad_phantasm_source_removed_before_resolution
Source ticket: mirror_mad_phantasm-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::mirror_mad_phantasm_source_removed_before_resolution
Scenario: Player A activates Mirror-Mad Phantasm's ability. Before the ability resolves, Player B exiles Mirror-Mad Phantasm. When the ability resolves, the creature is in exile and cannot be shuffled into the library. The "If that player does" conditional is false, so no reveal/mill occurs. Verify the library is unchanged.

### skirsdag_ability_should_use_stack
Source ticket: skirsdag_high_priest-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::skirsdag_ability_should_use_stack
Scenario: Place Skirsdag High Priest and two other creatures on the battlefield with morbid active. Activate the ability. Verify the ability goes on the stack and does NOT create the token until the ability resolves. An opponent should receive priority after the ability is placed on the stack.

### skirsdag_tap_cost_paid_before_stack
Source ticket: skirsdag_high_priest-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::skirsdag_tap_cost_paid_before_stack
Scenario: Place Skirsdag High Priest and two other creatures on the battlefield with morbid active. Activate the ability. Verify the two chosen creatures are tapped as part of cost payment (before the ability goes on the stack), not during resolution. If the ability is countered, the creatures remain tapped but no Demon token is created.

### skirsdag_summoning_sick_creature_can_be_tapped
Source ticket: skirsdag_high_priest-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_activated_no_stack_02.rs::skirsdag_summoning_sick_creature_can_be_tapped
Scenario: Place Skirsdag High Priest (not summoning sick) with two other creatures, one summoning sick. Set morbid active. Verify the summoning-sick creature appears as a valid tap candidate — the {T} restriction applies only to the High Priest itself (CR 302.6), not to the other creatures tapped as an additional cost.

## Also closes

- merged-activated-no-stack-01

## Test Run Results

- **test_back_from_the_brink_can_be_responded_to** — confirmed
  - test fn: `test_back_from_the_brink_can_be_responded_to`
  - assertion: CR 602.2a: activated ability should be on the stack; token created only on resolution
- **test_kessig_wolf_run_pump_can_be_responded_to** — confirmed
  - test fn: `test_kessig_wolf_run_pump_can_be_responded_to`
  - assertion: CR 602.2a: Kessig Wolf Run pump should not apply until ability resolves from stack
- **test_nephalia_drownyard_mill_can_be_responded_to** — confirmed
  - test fn: `test_nephalia_drownyard_mill_can_be_responded_to`
  - assertion: CR 602.2a: Nephalia Drownyard mill should not occur until ability resolves from stack
- **test_tree_of_redemption_exchange_can_be_responded_to** — confirmed
  - test fn: `test_tree_of_redemption_exchange_can_be_responded_to`
  - assertion: CR 602.2a: Tree of Redemption exchange should not occur until ability resolves from stack
- **full_moons_rise_regenerate_can_be_responded_to** — confirmed
  - test fn: `full_moons_rise_regenerate_can_be_responded_to`
  - assertion: CR 602.2a: regeneration shields should not apply until ability resolves from stack
- **mirror_mad_phantasm_opponent_can_respond** — confirmed
  - test fn: `mirror_mad_phantasm_opponent_can_respond`
  - assertion: CR 602.2a: Mirror-Mad Phantasm's activated ability should be on the stack
- **mirror_mad_phantasm_source_removed_before_resolution** — confirmed
  - test fn: `mirror_mad_phantasm_source_removed_before_resolution`
  - assertion: creature in exile cannot be shuffled into library; no reveal/mill should occur
- **skirsdag_ability_should_use_stack** — confirmed
  - test fn: `skirsdag_ability_should_use_stack`
  - assertion: CR 602.2a: Skirsdag High Priest should not create Demon token until ability resolves
- **skirsdag_tap_cost_paid_before_stack** — confirmed
  - test fn: `skirsdag_tap_cost_paid_before_stack`
  - assertion: CR 602.2a: Demon token should not be created until ability resolves from stack
- **skirsdag_summoning_sick_creature_can_be_tapped** — confirmed
  - test fn: `skirsdag_summoning_sick_creature_can_be_tapped`
  - assertion: CR 602.2a: Demon token should not exist until ability resolves from stack
