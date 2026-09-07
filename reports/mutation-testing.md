# Mutation testing: first engine-core run — 2026-08-29

Setup: `cargo-mutants` 27.1.0, scoped to the rules engine (`-p mtg-engine
-e isd` — per-card files under `src/cards/isd/` are out of scope, being
audited and acceptance-tested card by card). The full engine core holds
~2,365 mutants; `.github/workflows/weekly-mutants.yml` sweeps all of them
every Saturday across ten shards. This report covers the first deep run:
the four most load-bearing files (`sba.rs`, `combat.rs` — which as a glob
also matched `engine/actions/combat.rs` and `triggers/collect/combat.rs` —
`destruction.rs`, `stack.rs`), tested against the `mtg-engine` suite.

## Baseline numbers

320 mutants: **190 caught, 11 timed out** (mutants that livelock a game —
effectively caught), **92 unviable** (don't compile; `warnings = "deny"`
does real work here), **26 missed**. Catch rate on viable mutants: **88%**.

## The 26 survivors, triaged

**Killed by new tests:**

| Survivors | Gap | New test |
|---|---|---|
| `eligible_blockers` 3× `&&`→`\|\|` | no test asserted each clause of "untapped creature of the defender" separately | `combat_rules.rs::eligible_blockers_is_untapped_creatures_of_the_defender_only` |
| `deal_damage_step` 306 first-strike gate | the two-step damage flow was tested for attackers, never for a plain first-striker's once-only or double strike's twice | `a_plain_first_striker_deals_its_damage_exactly_once`, `a_double_striker_deals_damage_in_both_steps` |
| `deal_damage_step` 345 blocker first-strike gate | the first-strike-blocker test passed *vacuously* under this mutant — the dead blocker's `damage_marked` clears on the zone change, so asserting it zero proved nothing (caught by the verification run below) | `a_first_strike_blocker_kills_before_the_attacker_strikes_back`, strengthened to assert the blocker survived |
| `deal_damage_step` 308 blocked-ness check | nothing pinned CR 509.2's "blocked forever": a blocker *removed from combat* (regeneration, control change) empties the assignment while the attacker stays blocked; a blocker that merely dies stays in the snapshot, so the obvious repro never reaches this branch (caught by the verification run below) | `combat_rules.rs::a_blocked_attacker_whose_blocker_left_combat_hits_nobody` |
| `deal_damage_step` 371 `-`→`+` | lethal-damage calculation ignored damage already marked | `lethal_assignment_counts_damage_already_marked` |
| `walker_still_there` guards ×2 (312, 395) | no test for trample overflow at a *departed* planeswalker (CR 510.1c: it lands nowhere, not on the player) | `planeswalker_combat.rs::trample_overflow_lands_nowhere_when_the_walker_left` (+ blocked variant) |
| `sba.rs` 213 `&&`→`\|\|` | a state trigger already on the stack could re-trigger unpinned | `state_based_actions.rs::a_state_trigger_on_the_stack_does_not_retrigger` |
| `engine/actions/combat.rs` 31 `==`→`!=` | the walker-attack dedup ("drop an entry whose attacker already attacks the player") only diverges when one declaration mixes attacks on the player and on a walker; the forced-attacker test never touched this filter (caught by the verification run below) | `planeswalker_combat.rs::attacking_a_walker_alongside_attacks_on_the_player` |
| `triggers/collect/combat.rs` 54 `==`→`!=` | the equipment-bystander test asserts a *negative*, which an inverted zone filter also satisfies — and no Innistrad card declares `AnyCreatureAttacks`, so the watcher scan had no positive coverage at all (caught by the verification run below) | `trigger_dispatch.rs::an_attack_watcher_hears_the_attack_from_the_battlefield_only` (registers a test-only watcher card) |

**Accepted, with reasons (16 mutants, 13 normalized lines in `reports/mutants-accepted.txt`):**

- `stack.rs` graveyard re-check arms (10 mutants — deleted arms and
  operator flips in `is_target_legal`'s graveyard requirements, including
  the zone-table `==`→`!=` at 67, whose in-code comment already documents
  the masking). These
  clauses re-check properties of a graveyard card that are *immutable* —
  its owner, its printed creature-ness, its printed subtype — so a target
  the engine legally offered can never become illegal on those axes, and
  no sequence of legal play distinguishes the mutant. The arms are
  defense-in-depth against buggy target *offers* (the historical bug the
  code comment records) and against injected targets; two of the eight
  ("GraveyardCard", "OwnedByTargetPlayer") even fall through to an
  identical `_ => true` default. Killing them would mean testing through
  deliberately corrupted internal state for no behavioral payoff.
- `triggers/collect/combat.rs` zone guards (21, 81, 118). Declaration and
  trigger collection happen inside one action; nothing can remove the
  attacker or blocker between the event and the scan, so the guards are
  defensive and the mutants unreachable through legal play.
- `combat.rs` 122 `>`→`>=`: recording a minimum-blocker requirement of 1
  changes nothing — a single blocker always satisfies it.
- `combat.rs` 296 `\|\|`→`&&` (`was_blocked` snapshot-vs-live): the two
  sets diverge only mid-step in ways with no observable difference.
- `destruction.rs` 189 `\|\|`→`&&` in `death_event`: the right-hand
  fallback exists for registry-less callers; every engine path passes the
  registry, making the clause redundant there.

## How to read this instrument

- Missed mutants are findings, not failures — the weekly workflow stays
  green and publishes survivors as artifacts; this file is where survivors
  get triaged into "test gap" (fix) or "equivalent/defensive" (accept,
  with the reason written down).
- "Unviable" is not waste: 29% of mutants failing to compile is
  `warnings = "deny"` and the type system doing free mutation-catching.
- The 11 timeouts are livelock mutants — a broken SBA or combat loop hangs
  a game. The fuzzer's runner would also catch these; in mutation runs the
  120s timeout counts them as caught-by-hang.

## Verification

Re-running the same 320 mutants after the nine new tests: **203 caught,
12 timed out, 85 unviable, 19 missed** (319 recorded — the final mutant's
log write hit a full disk; the unrecorded one, `stack.rs:67` `==`→`!=`,
is in the accepted graveyard-arm family above). Some baseline "unviable"
outcomes shifted to "caught" between runs — build-order sensitivity in
cargo-mutants, not a suite change.

The instrument audited itself: four of the baseline's claimed kills were
false, and the 19 survivors exposed them. Two tests passed vacuously under
their mutants (345: cleared `damage_marked` after a zone change; 54: a
negative assertion an inverted filter also satisfies), one targeted the
wrong condition (31), and one gap needed a path no test drove (308:
removal from combat vs. death). All four are re-fixed with tests verified
to FAIL under manual application of their exact mutants — a check worth
keeping: a mutation-motivated test isn't done until it has been watched
killing its mutant.

Final state: the 15 remaining recorded survivors (plus the one unrecorded)
are exactly the accepted list. One caveat on the accepted-list format: the
workflow strips line:col, so an accepted line like "`\|\|`→`&&` in
deal_damage_step" masks *any* such mutant in that function (296 accepted,
but a regression re-surfacing 345 would be masked too). The price of
edit-stable comparisons; the per-line reasons above are the record.

# The full engine-core sweep — 2026-08-30

The first scheduled weekly run (all 2,367 engine-core mutants, run
33255670070) filed nine per-shard survivor issues (#26–#34). A local
full sweep against the same-day HEAD gave the exact picture: **1,125
caught, 32 timed out** (livelocks the suite catches by hanging), **789
unviable, 421 missed** — 241 unique survivors after normalization, 12 of
them already on the accepted list.

## Dispositions

Three buckets now exist, and every survivor is in exactly one:

1. **Killed** — a test written for it, watched failing under the exact
   mutant before it counts. This round: the composite branches of
   submitted-target validation, the CardBehavior hook defaults and the
   DFC name fallback, the auto-tap planner contract, X-funding
   arithmetic and bounds, the targeted-pump accumulation, same-controller
   control changes, the stack-entry accessors, printed colors
   (CR 202.2/204.2) — and the whole `invariant_checker.rs` battery: the
   fuzzing oracle's ~24 invariant families each verified to flag their
   corruption, with a rich clean state (populated libraries, graveyards,
   stack, attachments, loyalty, combat) pinning the false-positive
   direction that corruption tests alone cannot see.
2. **Accepted** (`reports/mutants-accepted.txt`) — judged equivalent or
   out of scope, each under a written reason: RNG mixing internals
   (determinism is the contract, pinned by the replay check), a
   runner-facing helper no engine path calls, an effect variant nothing
   emits yet, display/log-text arms, an identity mutant, re-lookup of a
   failed key, and per-card files outside the engine core.
3. **Backlog** (`reports/mutants-backlog.txt`) — genuine gaps, kept
   visible and worked down by the daily fixer, but not re-filed by the
   weekly workflow. Deleting a line is the "killed" ceremony; moving one
   to accepted needs a reason here.

The weekly workflow now files an issue only for survivors in none of the
three buckets — i.e. *new* regressions — and its cargo-mutants version is
pinned (27.1.0), because mutant names render differently across versions
and the suppression lists match on the rendered name.

## The checker battery, verified

A scoped re-sweep of `invariants.rs` after `invariant_checker.rs`:
**137 mutants — 110 caught, 15 unviable, 12 missed** (down from 120
missed in the full sweep). The clean-state half did most of the work:
an inverted check flags healthy structures, so the clean state carries
populated libraries, graveyards, a stack spell, attachments, loyalty,
declared combat, a damaged-but-healthy creature, a +1/+1 counter, and a
legend with its twin in the graveyard. Four of the twelve stragglers
were then killed directly (deathtouch/damage conjunction, the legend
skip clause, the counter-annihilation conjunction, the three-queue
trigger sum), verified failing under their mutants.

The last eight collapse to three normalized backlog lines: the
`being_cast` identity check (needs a mid-cast state), and the
`check_settled` skip-clause operator flips — several of which diverge
only on states already violating a different invariant or on printed
characteristics the pool doesn't contain (0-toughness cards, a printed
power without a toughness), which is why they sit in the backlog for a
closer look rather than the accepted list.

## Weekly sweep, run 33964071700

The weekly workflow shards the whole engine core across ten jobs and files
one issue per shard. Triage of the small shards:

**Shard 0 — `GameState::change_life`, two survivors on one comparison.**
The verb test `if delta > 0 { "gained" } else { "lost" }` runs after an
early return on `delta == 0`.

- `replace > with ==` makes the test always false, so every life change in
  the log reads as a loss. Killed by
  `a_life_gain_is_logged_as_a_gain_with_the_resulting_total`, which pins
  the gain line, the resulting total, and that no loss line is written.
  The suite had a test for the loss half (#129) and none for the gain.
- `replace > with >=` is equivalent: the delta cannot be zero here, so the
  two comparisons name the same set. Accepted.

**Shard 1 — the partial-fizzle line for an ABILITY.** `!newly_illegal
.is_empty()` guards the "target X is illegal, resolving with the rest"
line, and deleting the negation survived: every clean resolution would
have announced an empty list of illegal targets, and a real partial fizzle
would have said nothing. The suite pinned the SPELL half (#135) and never
the ability half, in either direction. Killed by
`an_ability_that_keeps_its_target_announces_no_fizzle`, which pins the
quiet case — an ability whose target stays legal announces nothing.

**Shard 4 — the cast-path target offer, four survivors in two places.**

- `options.len() < second_min` drops a first target with no legal pairing.
  Both `<=` and `==` mutations survived: each skips the case where a
  MANDATORY second slot has exactly one option, so Prey Upon with one
  creature a side would have offered nothing at all. Every existing test of
  this shape used "up to N", whose `second_min` is 0, where the comparison
  cannot be told apart. Killed by
  `a_mandatory_second_slot_with_one_option_is_still_offered`.
- The `*p == caster` sort key that puts the chooser first (issue #138)
  survived being replaced by either constant: with all entries keyed alike
  the stable sort falls back to seat order, which agrees with caster-first
  exactly when p0 is casting — and every test of the list cast from p0. The
  ability path had a both-seats test; the cast path had none. Killed by
  `a_spells_player_target_list_puts_the_caster_first`.

**Shard 2 — the trigger-order prompt, the view, and one equivalent mutant.**

- `process_pending_trigger_pushes` tags a *repeated* option in a CR 603.3b
  ordering prompt with its source's P/T and object id (issue #116).
  Deleting the `(Some(p), Some(t))` arm drops the P/T, and `> 1` → `>= 1`
  tags every option including distinguishable ones. Nothing pinned the
  tail's shape or its absence. Killed by
  `simultaneous_triggers_are_ordered_by_their_controller` (now asserting
  `[source 3/2, #<id>]` on each option) and by
  `distinct_trigger_options_are_not_given_a_source_tail`.
- `GameView::for_player` had four unpinned pieces of what the player is
  shown: the loyalty-ability label's sign (`loyalty_change > 0`, three
  surviving comparisons), the full log's level floor
  (`e.level > LogLevel::Private`, three more), Nevermore's chosen name
  (`PreventCastingNamed` arm), and the first-strike damage step flag
  (`step == CombatDamage && combat_damage_step_pending`). The same
  `combat_damage_step_pending` guard names the step in `legal_actions`'
  prompt context and survived being forced either way. All killed by four
  tests in `harness_display.rs`; see
  `the_view_and_the_prompt_name_which_combat_damage_step_this_is`.
- `CardBehavior::self_static_pt_mod -> Some((0, 0))` is equivalent. The
  trait default returns `None`, and both call sites
  (`effective_power`/`effective_toughness`) do nothing but `power += p` on
  a `Some`, so `Some((0, 0))` and `None` are indistinguishable by any
  observation the engine can make. Accepted.

**The loyalty offer gate.** `legal::abilities::loyalty` decides whether a
minus ability is payable with one condition, and its boundary was
unpinned: at three counters Liliana's -2 is payable and her -6 is not, and
at two her -2 costs exactly what is there (CR 118.3 forbids only going
below zero). Killed by
`a_minus_ability_is_offered_down_to_its_last_counter`, which pins all
three points. `replace < with <=` on the same line is equivalent — it only
changes the zero-cost case, where `-0` is `0` and `0` is never greater
than a `u32` loyalty count, so neither reading takes the branch. Accepted.

## The invariant checker, self-tested (issues #274–#279)

Six of the weekly sweep's ten shards landed on `mtg-engine/src/invariants/`
— ~390 surviving mutants across `mod.rs`, `transition.rs`, `turn.rs`,
`events.rs`, `prompts.rs`, `stack.rs`, `legal.rs`, `objects.rs`,
`permanents.rs` and `effects.rs`. They are all the same finding.

The checker is the fuzzing oracle: ~110k invariant-checked games run
nightly, and the fuzzer reports only what the checker reports. So a mutant
that blinds one clause is invisible — the games still pass, and the clause
stops being a clause. What the sweep found is that most of the checker's
clauses had a message and no state that produced it: `invariant_checker.rs`
and `invariant_families.rs` covered the families, but only a fraction of
the individual clauses inside them.

The fix is one test per clause, in the shape those two files already used:
build a healthy state, corrupt exactly one property, assert the message.
Where a clause is conditional — a scope test, a stand-down, a disjunction —
the neighbouring state that must NOT produce the message is asserted too,
because that is the half a mutant flips.

Four new files carry the clauses that did not fit the existing two:

| file | family |
| --- | --- |
| `invariant_event_window.rs` | `events.rs` — what this action's events say the next decision point must look like |
| `invariant_legal_offers.rs` | `legal.rs` — the menu offers exactly the game the rules allow |
| `invariant_prompt_shapes.rs` | `prompts.rs` — a prompt is answerable, addressed right, and asks about things that are there |
| `invariant_object_shapes.rs` | `objects.rs`/`permanents.rs`/`effects.rs` — what an object may look like in each zone |

Two suite guards (`no_test_assembles_combat_state_by_hand`,
`no_test_ends_the_turn_by_hand`) now share one list of the invariant
self-tests. Both guards exist to stop an ordinary test standing in for the
engine; an invariant self-test writes exactly those fields on purpose,
because the state the engine would never leave behind is the thing it is
asking the checker about.

# What to fix, and what not to — 2026-09-07

The first two months of this campaign ran on an unstated rule: a survivor
is a defect, so kill it. That rule is wrong, and following it started to
cost more than it returned.

`docs/mutation-testing-guide.md` is the replacement. The short version:
a survivor is a lead, the question is whether an engine user could see
anything go wrong if the edit shipped, and the second question is whether
the test that kills it would survive a legitimate refactor. A survivor
that fails the first goes on the accepted list with a reason; a cluster
that fails the second wants one property over the computation's output,
not one assertion per line.

The audit that produced it, over what this campaign has pinned so far:

- **The invariant self-tests (162 tests, six files) hold up.** They are
  the highest-value case in the guide's "fix" list — the checker is the
  only oracle over ~110k games a night, so a blinded clause silently
  removes a whole class of bug from the fuzzer's reach and the run stays
  green. They are also written at the right level: corrupt one property
  of a healthy state, assert the message. That survives a rewrite of the
  clause; it pins what the clause is for.
- **The tap planner's arithmetic should not have been on the list at
  all.** Twenty-three survivors inside `compute_autotap`'s internal
  simulation, in a function whose choice among correct plans is a
  heuristic that has been retuned twice. Replaced with two property
  cases over the contract (an offered plan pays; a declined one had no
  plan), which catch the same class and leave the heuristic free. This is
  the worked example in the guide.
- **The dozen older autotap tests that pin an exact source choice are
  borderline and stay.** Each cites a reported bug — #114 stranding a
  castable spell, #252 offering a plan the payment could not run — so the
  preference they pin is a symptom someone actually hit. New ones should
  assert the symptom (the spell is still castable) rather than the
  ordering.
- **Three of the "fixes" were deletions, and they were the best ones.**
  `generate_ability_targets` was a second copy of the requirement match
  that knew nine of seventeen words; `valid_targets_for_mode` was
  `valid_targets_for_req` with a wrapper peeled off; `replacement::apply`
  had a branch no caller could reach. Deleting removed the survivors, the
  drift, and the reader's confusion at once. The guide now names this as
  a disposition of its own.
- **Six backlog lines named functions that no longer exist** (the
  `try_auto_pay` arithmetic moved into `try_auto_pay_with_order` when the
  reserving order was split out). Stale lines cost a re-derivation every
  time someone reads the list; they are deleted on sight.

What did not change: the ceremony. A mutation-motivated test still is not
done until it has been watched failing under its exact mutant, because an
early round of this campaign produced four kills that were not kills.

## Auditing the campaign's own tests — 2026-09-07

`docs/mutation-testing-guide.md` was written after most of this campaign's
tests, so the tests were read back against it. Ninety-odd
mutation-motivated tests; six failed and were deleted, one function went
with them, and one was replaced at the right level.

**Deleted, for pinning a heuristic:**

- `flexibility_counts_the_colors_a_source_can_make_and_not_colorless`
- `a_sources_score_is_the_demand_for_every_color_it_makes`
- `hand_demand_adds_up_every_pip_in_every_cost`

All three assert the autotap planner's ranking inputs and the sort order
they produce. Nothing about a plan is right or wrong because of them — the
planner chooses among plans that are all correct, and the guide's own
worked example says so in as many words. Writing them was the same
mistake the guide was written to stop, made in the same session that
wrote the guide.

**Deleted, for pinning presentation:**

- `funding_groups_come_back_in_a_stable_order` — a funding response names
  its groups by NAME, so their position carries nothing. Any order the
  comparator produces is deterministic, which is the only property the
  replay check needs.
- `a_tap_plan_is_shown_as_a_short_grouped_list` — the exact rendering of
  "tap 2x Swamp, Sol Ring", down to where the runs break. Which sources a
  plan taps is the rules question, and it is pinned where the planner
  lives. (This one also carried a `let _ = name_b;` to silence a warning
  about a binding it no longer needed — a test contorting itself around
  its own subject.)

**Deleted, for restating a getter:**

- `a_triggers_kind_and_chosen_targets_are_readable` constructed a
  `PendingTrigger` by hand and asserted two accessors returned what had
  just been put in. Following it found something better than a test:
  `PendingTrigger::kind()` had **no callers at all** — every site calls
  `trigger.event.kind()` directly — so the wrapper is deleted.
  `chosen_targets()` turned out to be live but genuinely uncovered:
  stubbing it to return nothing passed the entire suite. Its two callers
  are the stack panel (issue #134) and the log line naming a trigger's
  target (issue #135), so it is now covered at that surface instead —
  `the_stack_view_shows_what_a_trigger_is_pointed_at`, from both seats,
  because the stack is public (CR 400.2).

**Kept, on the line:**

- `a_spells_player_target_list_puts_the_caster_first` pins an ordering,
  which the guide usually calls arid — but issue #138 is a filed bug about
  exactly that order (the chooser reads "You / Opponent", and the order
  flipped with the seat). The order is a UX contract someone hit.
- `the_step_and_card_type_predicates_say_what_they_mean` is a table of CR
  facts (502/514 on priority, 110.4a on permanent types) rather than a
  restatement of a card's own data.
- `a_source_produces_colorless_only_if_an_ability_says_colorless` sits
  beside the three deleted scoring tests but is not one of them: whether a
  source can make {C} decides whether a plan is *correct* (CR 107.4c),
  not which of several correct plans is preferred.
- The exact-string log assertions in `log_attribution.rs` stay. The log is
  a deliverable there — its header says so — and the issues behind those
  tests (#86, #263, #264, #299, #301) are all "the log could not answer an
  obvious question".

The mutants the deleted tests had been killing are on
`reports/mutants-accepted.txt` with the reason each is arid.
