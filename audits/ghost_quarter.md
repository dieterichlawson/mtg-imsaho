## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/240/ghost-quarter?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
```

**Status**: ISSUE

### Code issues
See below.

- Its ruling is the plain statement of CR 608.2b for abilities, and
  `stack.rs`'s `StackEntry::Ability` arm had no legality check at all.
  - Ruling (2011-09-22) says: `If the targeted land is an illegal target by the
    time Ghost Quarter's ability resolves, it won't resolve and none of its
    effects will happen. The land's controller won't get to search for a basic
    land card.`
  - Code did: resolve against whatever it had targeted, however the board had
    changed
  - Fixed: the arm now substitutes `Target::Illegal` for a target that can no
    longer be targeted or no longer satisfies the card's `is_valid_target`, and
    fizzles when every target is illegal.

- The ability's effect lived in `on_activate_ability`, whose trait default *was*
  the CR 602.2a stack push, so the effect happened the instant the ability was
  activated and no opponent ever received priority.
  - CR 602.2a says: `the ability goes on the stack`
  - Code did: `fn on_activate_ability(&self, ...) { <the effect> }` — overriding
    the push away
  - Fixed set-wide: the hook is gone, the engine owns the push
    (`engine::actions::abilities::put_ability_on_stack`), and the effect moved to
    `resolve_activated_ability`. See
    `reports/ISD_AUDIT_CR6022a_ACTIVATED_ABILITIES.md`.

### Tricky interactions checked
- "Its controller **may** search" — the land's controller is offered the choice,
  and a player who declines does not shuffle: PASS
- The controller is offered the search even with no basic to find (declining is
  still their call): PASS
- The found land goes to the battlefield untapped, not to hand: PASS
- Sacrificing Ghost Quarter is a cost, paid on activation, so it is in the
  graveyard while the ability is on the stack: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- CR 602.2a (the ability waits on the stack): `activated_no_stack.rs:activating_through_the_engine_leaves_the_ability_on_the_stack`
- CR 608.2b (targets re-checked on resolution): `fizzle.rs:an_activated_abilitys_targets_are_rechecked_when_it_resolves`
- Guards: `test_suite_guards.rs:no_card_or_test_names_the_removed_activation_hook`, `test_suite_guards.rs:only_the_engine_puts_an_ability_on_the_stack`
- The "may" choice with no basics: `resolution_time_checks.rs:ghost_quarter_may_choice_offered_when_no_basics`
- Declining does not shuffle: `resolution_time_checks.rs:ghost_quarter_declining_the_search_does_not_shuffle`
- Searching does shuffle: `lands_and_mana.rs:ghost_quarter_shuffles_the_library_after_the_search`
