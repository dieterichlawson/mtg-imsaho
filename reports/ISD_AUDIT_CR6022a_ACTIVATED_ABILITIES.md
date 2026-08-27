# Activated abilities never waited on the stack (CR 602.2a)

Found while auditing Ghost Quarter, and the largest single-rule gap the
Innistrad audit has turned up. It spanned three layers; all three are fixed.

## The rule

CR 602.2a: activating an ability puts it on the stack. It does not happen yet.
Every player receives priority first (CR 117.3b), and the ability can be
responded to, countered, or made to fizzle before any of its effect lands.

CR 602.2b sends the rest of activation through the spell-casting steps, so
costs (CR 601.2h) are paid with the ability already on the stack.

Ghost Quarter's ruling is the plain statement of the consequence:

> If the targeted land is an illegal target by the time Ghost Quarter's ability
> resolves, it won't resolve and none of its effects will happen. The land's
> controller won't get to search for a basic land card.

## Layer 1 — the engine resolved the ability immediately

`engine/actions/abilities.rs`, in both the X-cost and ordinary branches, did:

```rust
behavior.on_activate_ability(&mut *state, object_id, ability_index, targets, registry);
if state.stack.last().is_some_and(|e| matches!(e, StackEntry::Ability { .. })) {
    crate::stack::resolve_top_of_stack(&mut *state, registry);
}
```

Push and resolve in the same breath, so the stack entry existed for the length
of one function call and no opponent could ever respond to an activated ability.

**Fixed** by deleting both immediate-resolve calls (and the third in
`choices.rs`'s X-funding continuation). The priority loop in `engine.rs` already
resolves the top of the stack when every player has passed — abilities now go
through it exactly as spells do. `ActivateAbility` also resets
`consecutive_passes` (CR 117.3b), which it did not need to when the resolution
was immediate.

## Layer 2 — 46 of 53 cards deleted the push

`CardBehavior::on_activate_ability`'s *default body was the stack push*. Cards
were told to override it "to add card-specific cost payment before the stack
push", but 46 of the set's 53 activated abilities overrode it to do their
**effect**, silently removing the push. So:

- Ghost Quarter destroyed the land the instant the ability was activated.
- Elder of Laurels counted creatures at announcement, not at resolution —
  against its ruling, "the number of creatures you control is counted as the
  ability resolves."
- Heretic's Punishment and Olivia Voldaren hand-rolled their own target-legality
  checks inside the activation hook, because the engine's could never run.

**Fixed** by removing the hook entirely. `engine::actions::abilities::
put_ability_on_stack` owns the push; effects live in
`resolve_activated_ability`; a cost the `ActivatedAbilityDef` cannot express
goes in the new `pay_activation_cost` hook, which only Moorland Haunt (exile a
creature card from a graveyard) and Blazing Torch (sacrifice the Equipment
attached to the creature the ability was activated on) need.

Moving the push into the engine also fixed a bug the card-owned version could
not avoid: `behavior_card_id` is not always the activated object's own card.
Skeletal Grimace grants "{B}: Regenerate this creature" to what it enchants and
Blazing Torch grants its damage ability to what it equips, so resolution has to
dispatch to the *granting* card. Only `activate_ability` has done the
native → copy-grantor → attached-permanent walk that resolves it.

## Layer 3 — CR 608.2b was not checked for abilities at all

`stack.rs`'s `StackEntry::Ability` arm had no target-legality check, so an
ability resolved against whatever it had targeted however the board had changed.

**Fixed**: the arm now substitutes `Target::Illegal` for a target that can no
longer be targeted and fizzles when every target is illegal, matching what
`resolve_spell` already did.

## Guards

Two build-failing scanners in `test_suite_guards.rs`:

- `no_card_or_test_names_the_removed_activation_hook` — the name cannot come
  back, in cards or in tests.
- `only_the_engine_puts_an_ability_on_the_stack` — nothing outside
  `put_ability_on_stack` constructs a `StackEntry::Ability`.

## Tests

- `activated_no_stack.rs::activating_through_the_engine_leaves_the_ability_on_the_stack`
  drives the real action (`submit_action(ActivateAbility)`) rather than the
  hooks, and asserts the ability is on the stack, the effect has not happened,
  and the pass count was reset. Every other test in that file drove a seam the
  game did not go through.
- `fizzle.rs::an_activated_abilitys_targets_are_rechecked_when_it_resolves` was
  `#[ignore]`d on this finding — there was no window in which to make a target
  illegal. It now runs and passes.
- `common::activate_via_hooks` drives both halves for tests that set up a board
  `legal_actions` would not offer the ability from; `common::activate_onto_stack`
  stops at the stack for tests that need the window in between.
