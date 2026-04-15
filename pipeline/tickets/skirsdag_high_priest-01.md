---
id: skirsdag_high_priest-01
status: closed-duplicate
card: Skirsdag High Priest
card_file: mtg-engine/src/cards/isd/skirsdag_high_priest.rs
created: 2026-04-15T03:30:28Z
audit_run_id: 2026-04-14-skirsdag_high_priest-audit
audit_model: opus
audit_tokens: 12265
audit_duration: 287
duplicate_of: merged-activated-no-stack-02
---

## Audit Finding

**Oracle text:**
> Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.

**Code:**
> engine.rs:2717-2719: `behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);` — called immediately after cost payment, without placing the ability on the stack.
> skirsdag_high_priest.rs:80-131: `on_activate_ability` both taps the two chosen creatures (lines 112-115, an activation cost) and creates the Demon token (lines 118-127, the effect) in the same function call.

**Description:**
Activated abilities in this engine bypass the stack entirely — `on_activate_ability` is called immediately during `apply_action`, so the effect resolves without going through the stack. Per CR 602.2–4, an activated ability is announced, costs are paid, then the ability is placed on the stack; players receive priority and can respond (e.g., Stifle, Trickbind, summary dismissal). Only when the ability resolves does the effect occur. Here, the 5/5 Demon token appears instantly with no window for opponent interaction. Additionally, the "Tap two untapped creatures you control" cost (everything before the colon) is paid inside `on_activate_ability` at the same time as the token creation effect, rather than at CR 602.2h before the ability goes on the stack. This means the cost and effect are temporally indistinguishable, and any "whenever a player activates an ability" or "whenever a creature becomes tapped" triggers that should fire between cost payment and resolution cannot interact correctly.

**Engine path:**
- engine.rs:2717-2719 (on_activate_ability called immediately, no stack)
- engine.rs:2651-2653 ({T} tap cost paid correctly by engine)
- skirsdag_high_priest.rs:112-115 (two-creature tap cost paid inside effect handler)
- skirsdag_high_priest.rs:118-127 (token creation, the actual effect)

**Required check:** 8c, 8i

**Affected cards:**
- Skirsdag High Priest
- All cards with activated abilities (engine-wide architectural limitation)

## Tests

### skirsdag_ability_should_use_stack
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Skirsdag High Priest and two other creatures on the battlefield with morbid active. Activate the ability. Verify the ability goes on the stack (state has a pending stack entry) and does NOT create the token until the ability resolves. An opponent should receive priority after the ability is placed on the stack, before it resolves.

### skirsdag_tap_cost_paid_before_stack
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Skirsdag High Priest and two other creatures on the battlefield with morbid active. Activate the ability. Verify the two chosen creatures are tapped as part of cost payment (before or at the moment the ability goes on the stack), not during resolution. If the ability is countered (e.g., by Stifle), the creatures should remain tapped (costs are not refunded) but no Demon token should be created.

### skirsdag_summoning_sick_creature_can_be_tapped
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Skirsdag High Priest (not summoning sick) on the battlefield with two other creatures, one of which IS summoning sick. Set morbid active. Verify the summoning-sick creature appears as a valid tap candidate in the ability's enumerated pairs — the {T} tap symbol restriction applies only to the High Priest itself (CR 302.6), not to the other creatures being tapped as an additional cost.
