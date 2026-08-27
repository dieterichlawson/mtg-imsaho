# Activated abilities never wait on the stack (CR 602.2a)

Found while auditing Ghost Quarter. This is the largest single-rule gap the
Innistrad audit has turned up, it spans three layers, and it is recorded here
rather than fixed because the fix is a change to the priority loop, not to any
card.

## The rule

CR 602.2a: activating an ability puts it on the stack. It does not happen yet.
Every player receives priority first, and the ability can be responded to,
countered, or made to fizzle before any of its effect lands.

`mtg-engine/tests/activated_no_stack.rs` already states this in its module doc,
so the intended design is not in question.

## Layer 1 — the engine resolves the ability immediately

`mtg-engine/src/engine/actions/abilities.rs`, in both the X-cost and ordinary
branches:

```rust
if let Some(behavior) = registry.get(behavior_card_id) {
    behavior.on_activate_ability(&mut *state, object_id, ability_index, targets, registry);
}
if state.stack.last().is_some_and(|e| matches!(e, StackEntry::Ability { .. })) {
    crate::stack::resolve_top_of_stack(&mut *state, registry);
}
```

The push and the resolution are adjacent. No player receives priority between
them, so **no activated ability in the engine can be responded to** — not by a
removal spell on the source, not by a protection effect on the target, not by
anything.

Measured on Ghost Quarter through the real `submit_action` path: after
activating its "destroy target land" ability, `state.stack.len() == 0` and the
land is already in the graveyard.

## Layer 2 — 46 of 53 cards bypass even the push

The trait's default `on_activate_ability` exists to do the stack push, and its
doc says so ("Called when a non-mana activated ability is activated (CR 602.2a).
Default pushes the ability onto the stack."). A card that overrides it and puts
its *effect* there instead has opted out.

Overriding is correct only for bespoke cost payment, and those cards implement
`resolve_activated_ability` as well — Skirsdag High Priest and Back from the
Brink do exactly that, and they are among the seven that are right.

The other 46 put the effect in `on_activate_ability` and implement no
`resolve_activated_ability`:

Avacynian Priest, Blazing Torch, Bloodline Keeper, Brain Weevil, Butcher's
Cleaver, Cellar Door, Civilized Scholar, Cobbled Wings, Darkthicket Wolf,
Daybreak Ranger, Demonmail Hauberk, Disciple of Griselbrand, Elder of Laurels,
Evil Twin, Feral Ridgewolf, Gavony Township, Ghoulcaller's Bell, Graveyard
Shovel, Grimgrin Corpse-Born, Grimoire of the Dead, Heretic's Punishment,
Inquisitor's Flail, Kessig Wolf, Lantern Spirit, Ludevic's Test Subject, Manor
Gargoyle, Manor Skeleton, Mask of Avacyn, Mikaeus the Lunarch, Mindshrieker,
Moorland Haunt, Olivia Voldaren, Runechanter's Pike, Selfless Cathar, Sharpened
Pitchfork, Silver-Inlaid Dagger, Silverchase Fox, Skeletal Grimace, Skirsdag
Cultist, Stensia Bloodhall, Stitcher's Apprentice, Traveler's Amulet,
Trepanation Blade, Ulvenwald Mystics, Wooden Stake.

(Ghost Quarter was the 46th and has been converted to `resolve_activated_ability`
as a worked example. It behaves identically today, because layer 1 resolves it
at once either way — the conversion is a step toward the fix, not the fix.)

## Layer 3 — the tests pass without covering the real path

`activated_no_stack.rs` calls the behaviour directly:

```rust
behavior.on_activate_ability(&mut state, wolf_run, 1, &[Target::Object(target)], &reg);
assert_eq!(state.effective_power(target, &reg).unwrap(), base_power,
    "CR 602.2a: ... should not apply until ability resolves from stack");
```

That exercises the trait default's push in isolation and never goes through
`engine::submit_action`, so it passes while the path a player actually takes
resolves the ability one line after pushing it. The file's assertions are about
the right rule; they are aimed at a seam the real game does not use.

## Rulings this makes unreachable

Several ISD cards have rulings that only mean something if an ability can be
responded to:

- **Ghost Quarter** — "If the targeted land is an illegal target by the time
  Ghost Quarter's ability resolves, it won't resolve and none of its effects
  will happen. The land's controller won't get to search for a basic land card."
- **Olivia Voldaren** — "If you activate Olivia Voldaren's last ability, and
  before that ability resolves you lose control of Olivia Voldaren, the ability
  will resolve with no effect."
- **Heretic's Punishment** — "If the targeted permanent or player is an illegal
  target by the time the ability resolves, the entire ability won't resolve."

None of these can currently be reached.

## What a fix looks like

1. Stop resolving the ability in `submit_action`; let the normal priority loop
   resolve the top of the stack, as it already does for spells.
2. Convert the 46 cards: move the effect from `on_activate_ability` to
   `resolve_activated_ability` and let the default push run. Mechanical for all
   of them, since the engine pays declared costs (`requires_tap`,
   `sacrifice_cost`) itself.
3. Re-point `activated_no_stack.rs` at `engine::submit_action` so it covers the
   real path, and un-ignore
   `fizzle.rs::an_activated_abilitys_targets_are_rechecked_when_it_resolves`.

Step 1 is the one with real blast radius: every test that activates an ability
currently assumes the effect has already happened when `submit_action` returns.
