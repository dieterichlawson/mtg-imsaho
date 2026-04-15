---
id: merged-activated-no-stack-01
status: closed-duplicate
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: back_from_the_brink-02, kessig_wolf_run-03, nephalia_drownyard-01, tree_of_redemption-03
duplicate_of: merged-activated-no-stack-02
---

# Activated abilities resolve immediately with no stack entry (CR 602.2a)

## Description
Per CR 602.2a, activating an activated ability places an instance of it on the stack; players receive priority and may respond before it resolves. The engine has no StackEntry variant for activated abilities. `engine.rs:2559-2725` (the `ActivateAbility` handler) pays costs and then immediately calls `on_activate_ability`, producing the effect atomically with activation. Opponents cannot respond to an activation window — no counterspell-like response, no Stifle, no sacrifice-for-value before removal. This is a known engine-wide architectural limitation affecting every non-mana activated ability.

## Engine path
- state.rs:10-15 (StackEntry enum — no Ability variant)
- engine.rs:2559-2725 (ActivateAbility handler — atomic activation + effect)
- engine.rs:2717-2719 (immediate dispatch to on_activate_ability)
- engine.rs:3199-3214 (ChooseXFunding — X-cost path also resolves immediately)

## Tests

### test_back_from_the_brink_can_be_responded_to
Source ticket: back_from_the_brink-02
Implementation: (not yet written)
Scenario: Activate Back from the Brink's exile-and-create-token ability. Verify the ability is on the stack (observable via stack inspection) and opponents receive priority before it resolves.

### test_kessig_wolf_run_pump_can_be_responded_to
Source ticket: kessig_wolf_run-03
Implementation: (not yet written)
Scenario: Activate Kessig Wolf Run's pump with X=3. Verify the ability is on the stack before the pump applies; opponent can respond (e.g., remove the target).

### test_nephalia_drownyard_mill_can_be_responded_to
Source ticket: nephalia_drownyard-01
Implementation: (not yet written)
Scenario: Activate Nephalia Drownyard's mill ability. Verify the ability goes on the stack and the target player receives priority to respond.

### test_tree_of_redemption_exchange_can_be_responded_to
Source ticket: tree_of_redemption-03
Implementation: (not yet written)
Scenario: Activate Tree of Redemption's exchange ability. Verify the ability is on the stack; opponents can destroy Tree in response, causing the exchange to fail (per Scryfall ruling).
