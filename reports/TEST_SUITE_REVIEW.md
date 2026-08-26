# Test suite review

A read of all 1,299 integration tests in `mtg-engine/tests/` (125 files, ~40k
lines) as of `25521e6`, looking for tests that should be combined, eliminated,
or fixed. This records what was found, what was done about it, and what was
deliberately left alone.

Result: **1,299 tests → 1,211**, with coverage going up rather than down —
most of the reduction is families of per-card tests replaced by sweeps over the
whole set, which cover strictly more cards than the hand-written lists did.

## The recurring problem

Almost every finding was a variant of one thing: **a test that passes for a
reason other than the behaviour it names.** Four shapes of it, in rough order
of how many tests were affected.

### 1. Restating the input as the expected output (48 tests)

Each card file was headed by a `*_card_data` / `*_has_correct_stats` test that
read the card's own `CardData` literal and asserted its fields straight back —
`power: Some(1)` in the card, `assert_eq!(data.power, Some(1))` in the test.
There is no independent source of truth for a card's printed values in this
repo, so these could only fail when somebody edited the card, and then they
failed without adding anything the diff had not already shown.

Replaced by `card_data_invariants.rs`: eleven checks on the *relationships*
between the fields, run across all 289 cards at once, so a newly registered
card is covered the moment it exists. Creature ⟺ has P/T; land ⟺ no mana cost;
Equipment ⟹ Artifact; Curse ⟹ Aura (CR 205.3h); flashback only on
instants/sorceries and never free (CR 702.33a); every declared keyword actually
printed on the card; nothing declared twice.

Each invariant also asserts how many cards it looked at, so one that stops
covering anything fails instead of passing vacuously — the same guard
`mana_filters.rs` already used on its tap-plan sweep.

### 2. Asserting only that nothing happened (14 tests)

Nine tests in `auto_pick.rs` were about a player getting to make a choice, and
every one checked only that the choice had *not* been made for them. Four wrote
it as an explicit escape hatch — `assert!(prompt_appeared || nothing_happened)`
— and five asserted only the negative half. **Deleting the ability outright
would have passed all nine.**

Same shape in `your_upkeep_scope.rs` (six tests asserting a trigger did not
fire on the opponent's step, which a card with no trigger also satisfies) and
`activated_no_stack.rs` (ten tests asserting an ability's effect had not
happened yet, which a broken ability also satisfies).

All now assert both halves. Where an implementation could plausibly have picked
the first match, the tests answer with the *second* option and check the first
stayed put.

### 3. Calling a hook the card does not implement (5 tests)

`mirror_mad_phantasm_mills_to_find_itself` called `on_activate_ability`, but
the reveal-and-mill loop lives in `resolve_activated_ability`. It then asserted
the Phantasm was on the battlefield (where it already was) and that two library
cards were "in Graveyard or Library" — the only two zones they could be in.

`bug_ae_undead_alchemist_replaces_damage_not_restores_life` called
`on_any_combat_damage_to_player`, which Undead Alchemist stopped overriding
when it became a `replace_event` replacement, then asserted life was unchanged
after that no-op — true of every card in the game.

This is mechanically detectable, so `test_suite_guards.rs` now fails the build
on it. The guard found a fifth case the hand search missed.

### 4. Documentation that contradicts the test (137 comments)

62 tests carried "This test asserts the EXPECTED CORRECT behavior, so it
currently fails. It will start passing as soon as Bug X is fixed." Every one
was passing. A reader who believes them mistrusts a green suite, which is worse
than no comment at all.

75 comments pointed at a source line (`engine.rs:4085`). Nineteen already
pointed past the end of the file they named — one nearly 3,000 lines past it —
because the refactor moved everything and no line number survives an edit above
it. Line numbers removed, file names kept and now guarded.

## Families collapsed into sweeps

Seven places tested a rule by writing one test per card. Each such list covers
the cards somebody happened to type and silently misses the rest, including
every card added afterwards.

| Was | Now |
| --- | --- |
| 9 DFC zone-cleanup tests | every DFC, both graveyard and exile |
| 3 named vanilla creatures | every creature with no death ability |
| 6 equipment wrappers + a hand-written cost table | table read off each card's equip ability |
| 6 upkeep-scope tests | every controller-scoped step trigger, both directions |
| 7 werewolf transform tests | every werewolf, against its own back-face data |
| 6 + 3 SBA boundary tests | two tables where a missing case is visible |
| 7 indestructible tests across 2 files | one table, with the grant mechanism as an axis |

Two of these caught bugs the hand-written lists had missed:

- Expanding the equipment table from six cards to all of them turned up
  **Blazing Torch**, which legitimately grants the equipped creature an ability
  of its own. The duplicate-equip check had been counting every action aimed at
  the target, so it would have flagged that as a duplicate.
- The DFC sweep exposed that `has_keyword` is zone-gated by design, so the old
  graveyard keyword assertions passed no matter which face the object thought
  it had. The sweep asserts keywords on the way *back* to the battlefield
  instead.

## Duplicates folded together

Eight rules were tested in two or three files at once, usually because a
bug-audit test and a rules test grew up separately.

Exact duplicates (weaker copy removed): `bonds_of_faith_locks_non_human`,
`cannot_play_land_during_opponent_turn`, `equipment_detaches_when_creature_dies`,
`geist_creates_angel_on_attack`.

Fizzling had three homes — `fizzle.rs`, `spell_fizzle.rs`, and a cluster in
`engine_regressions.rs`. `spell_fizzle.rs` is gone and its unique cases moved.
One of the copies dropped, `doom_blade_target_already_gone`, moved its target
to the graveyard and then asserted the target was in the graveyard — true
whether the spell fizzled or resolved.

`tapped_creature_cannot_block` existed twice but tests two different things
(eligibility, and that an illegal block absorbs no damage), so the second was
renamed rather than removed.

## Tests whose names were lies

- `bug_mirror_mad_phantasm_sets_draw_flag_incorrectly` abandoned the Phantasm —
  and its own first setup block — and actually exercised
  `PlayerState::reveal_top_card`.
- `garruk_back_face_tutor_shuffles_library` argued in a comment that a 1-in-24
  flake would be evidence of shuffling, while asserting only that the library
  got one card shorter. It now runs the tutor twenty times and requires the
  order to vary.
- `claustrophobia_prevents_untap` gave up mid-test ("Simpler: just run the
  untap logic directly"), abandoned its first fifteen lines, and rebuilt the
  state from scratch.
- `bug_burning_vengeance_spellcast_filter_excludes_creatures` ended in a
  comment — "Mark as FIXED" — and asserted nothing about the card.

## Re-filed

Two clusters sat in files named for cards while testing engine rules with a
card only as scenery: eight regeneration tests (CR 701.15) operating on a bare
permanent, now `regeneration.rs`; and four planeswalker tests (CR 306/606/704.5i)
using Liliana as a fixture, now `planeswalkers.rs`.

27 tests hand-wrote `obj.card_types = vec![CardType::X]` on a permanent placed
from the registry. That field holds *runtime grants*; printed types come from
the active face and the accessors already reported the right answer. Writing it
forced the answer the test wanted to see, which is exactly how a
characteristics-layer regression would slip past.

## Deliberately left alone

- **183 direct `behavior.on_*()` calls.** Most are legitimate unit tests of a
  card. Converting them all to drive the engine would be a very large change
  with real risk of behaviour drift, for modest gain. The dangerous subset — a
  call to a hook the card doesn't implement — is now guarded instead.
- **~60 tests still named `bug_*`.** The name describes a defect rather than a
  rule, but their bodies cite the audit ticket usefully, and renaming them all
  is churn without much payoff. Worth doing opportunistically.
- **Moorland Haunt and Traveler's Amulet apply their effects in
  `on_activate_ability`** rather than on resolution. For Moorland Haunt that
  looks wrong under CR 602.2a — the Spirit token should not appear until the
  ability resolves — but that is a card-behaviour change, not a test change,
  so it is noted here rather than made.

## Guards added

`test_suite_guards.rs` fails the build on all three of the rot patterns above,
so none of them can come back silently:

- no comment may claim a passing test is failing (a test that genuinely should
  not pass yet belongs behind `#[ignore]`, where the runner reports it);
- every source file a comment names must exist;
- no test may call a card hook the card leaves at its trait default.
