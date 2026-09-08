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

### Second pass: the tests the campaign did not write

The first audit found the mutation campaign's own tests by their commit
messages, which missed two things: the campaign's earlier rounds, whose
commits do not say "mutant", and the planner's ORIGINAL tests, which
predate the campaign and fail the same standard.

Three routes found the full set — commits touching the mutants ledgers, a
scan for test comments naming the instrument ("shard 2", "survived the
suite", "nothing pinned"), and the campaign window. 88 tests outside the
six invariant self-test files, plus those files entire.

**Eleven more deleted.** Ten are the autotap planner's original tests, and
they say what they are in their own comments: "Should tap Forest, not
Harbor", "Should tap Plains", "Should use Forest and Island, not Harbor".
Every one names which of several correct plans the planner picks.
`autotap_dual_for_generic` is the clearest case — its comment argues with
itself mid-sentence ("hand needs U less... actually hand needs U more.
Wait:") before landing on whatever the implementation did.

The eleventh, `distinct_trigger_options_are_not_given_a_source_tail`,
asserts a prompt row does NOT grow a disambiguating tail when it is
already distinct. The half that matters — two byte-identical rows DO get
one, so a player can tell which trigger they are ordering (issue #116) —
is asserted elsewhere. Adding a tail where it is not needed is noise, not
wrongness.

**Two of the ten were replaced, not just dropped.** Issue #114 is a real
symptom: a tap plan stranded a spell the remaining sources could have
paid for. `a_plan_does_not_strand_a_spell_the_rest_of_the_board_could_pay_for`
states that, over three boards, by asking the planner for a second plan
off whatever the first left untapped. It survives any retune of the
heuristic; the tests it replaces did not.

**No coverage was lost.** Re-sweeping after the deletions: 111 of 131
flips in `compute_autotap` still caught (the same 20 survivors as before,
all already accepted bar two), and 60 of 60 across `free_abilities_first`,
`ability_cost`, `ability_total_mana`, `ability_producing`,
`can_produce_color`, `can_produce_colorless` and `source_flexibility`. The
ten preference tests were killing nothing the two property tests do not.

Running total: 17 tests deleted, one dead function (`PendingTrigger::kind`)
deleted with them, two replacements written at the level the guide asks
for.

## The cast clause's target legality (issue #276, events.rs)

`spells_and_lands` reads a cast spell's chosen targets and asks whether the
spell could have pointed at them — hexproof on a creature (CR 702.11b), on a
player (CR 702.11c), protection from the spell (CR 702.16b). It is the
checker's only look at target legality after the fact, so a clause that goes
quiet there takes a whole class of illegal cast out of ~110k fuzzed games a
night while the run stays green. That is the "oracle clause" bucket, and the
highest value per test in this repo.

Four survivors were re-run against current master and triaged:

- **`replace && with || in spells_and_lands`** (the hexproof arm's
  "and controlled by an opponent"). Under `||` every spell pointed at an
  opponent's creature — the commonest legal cast in the game — is reported
  as targeting something with hexproof. It survived because no self-test
  targeted an opponent's creature at all: the existing cast test pumps the
  caster's own bear. Killed by
  `a_cast_events_targets_are_ones_the_spell_could_have_chosen`, whose first
  assertion is that the ordinary case passes in silence.
- **`replace match guard … with false`** and **`replace != with ==`** on the
  player arm. The first blinds CR 702.11c entirely; the second asks whether
  the *caster* has hexproof instead of the target. Witchbane Orb is in the
  pool and is the one card that grants it, so both are reachable. Killed by
  `a_cast_event_may_not_target_a_player_with_hexproof`, which checks the
  violation fires for an opponent the Orb protects and does not fire for a
  player targeting themselves under their own Orb.
- **`replace && with || in spells_and_lands`** at the dead
  `if quiet && on_bf(state, *object) {}` — an empty block whose comment said
  "unreachable but keeps the shape symmetric". Deleted, which removes the
  mutant, the reader's confusion and the drift risk in one move.

### Accepted

`replace match guard on_bf(state, *tid) with true in spells_and_lands`

The guard narrows the target arm to objects still on the battlefield. Widened
to `true`, the arm also runs for a target that has left — and CR 400.7 makes
an object off the battlefield its printed self, so `has_keyword(Hexproof)`
and `has_protection_from` both answer false for it and no violation is
produced. Reaching a difference needs a printed hexproof or protection card
in a non-battlefield zone that the checker still sees named as a target,
which no card in this pool produces. Watched surviving under its own mutant
after the three kills above were in place.

# The invariant checker's own clauses, re-run and triaged — 2026-09-08 (issues #276–#279)

Weekly-mutants run 33964071700 filed 335 surviving mutants across four
issues, every one of them in `mtg-engine/src/invariants/` — the fuzzing
oracle. Filed lines carry no line:col (that is how the workflow's accepted
list survives unrelated edits), so they name 1301 mutants in the tree as it
stands; 15 more name code that no longer exists at all — `arity_ok` moved
to `engine/targeting.rs`, `copy_choice_live` and the `AddCounters` and
`CopyCreature` pending effects are gone — and are simply retired here.

The sweep that produced them ran against the tree of 2026-09-05; the
checker has had a battery of self-tests since (`## The invariant checker,
self-tested`), so the first question was how many were still alive.

## The re-run

`reports/mutants-rerun-2026-09-08.txt` is the raw record: one line per
mutant, `outcome<TAB>mutant`. Two passes:

1. **All 1301 of them against the nine test binaries that exercise the
   checker** (`combat_rules`, `control_durations`, `invariant_checker`,
   `invariant_event_window`, `invariant_families`, `invariant_legal_offers`,
   `invariant_object_shapes`, `invariant_prompt_shapes`,
   `resolution_time_checks`), in six shards of ~215 each. Narrowing the test
   set cut the per-mutant cost from ~30 s to ~7 s, which is what made
   re-testing all of them affordable at all. **929 caught, 367 missed, 5
   unviable** — so two thirds of what was filed a weekly run ago is already
   dead, killed by the self-test battery that landed after the sweep.
2. **Every one of those 367 against the whole `mtg-engine` suite**, because
   a miss against nine binaries is only a lead. 359 of them still exist
   under their filed identity; the other 8 had moved a line or two and were
   re-tested under their current one — 7 of them, at any rate: one was
   dropped by the remapping and is re-tested at the end of this write-up.
   Two passes, because tests were being written between them: **119 died to
   the first, 32 more to the second**, and **215 came out the other side**.

Three more passes followed (§ *The second round*, § *The third round*): a
fourth cargo-mutants run over everything still alive, a test written for
one site by name, and then the whole remaining backlog worked one mutant at
a time.

So of the 1301 mutants the filed lines name: **929 were already dead**
before this pass began, **346 died to the tests written in it** (119 + 32 +
132 + 3 + 60, one pass at a time), **21 are accepted** as equivalent or
unreachable with the reasons below and in
`reports/mutants-accepted.txt`, **none are left on the backlog**, and 5 do
not compile.

## What was killed, and why those

The productive bucket in the guide's terms is the third one — *an oracle
clause* — and the survivors sorted into two shapes:

- **A clause with no bad state behind it at all.** Its message existed and
  nothing in the suite produced it. Every one of these is worth a test: the
  clause is load-bearing by construction (it is the only thing that reports
  its own class of bug), and the test is the guide's non-brittle shape —
  build a healthy state, corrupt one property, assert the message.
- **A clause whose bad state was built once, for a conjunction or a
  disjunction of several conditions.** A chain like "turn one, and the first
  turn, and the untap step" is only tested by breaking each condition by
  itself; a test that breaks all three at once passes under a mutant that
  drops any one of them. These are worth a case each, and the case is cheap:
  the fixture already exists.

Both are the same underlying gap — the state that distinguishes the mutant
was never built — and both fixes read as tests of the rules, not of the
implementation.

### One vacuous test, found on the way

`invariant_object_shapes.rs::a_modal_spell_on_the_stack_chose_one_of_its_modes`
cast **Brimstone Volley**, which is not modal, and wrapped its whole body in
`if matches!(…, ModalChoice(_))` — a guard that was never true. The test
passed by doing nothing, and every CR 700.2 clause it claimed to cover was
untested; four mutants inside them survived every sweep since. It casts
**Ghoulcaller's Chant** now, which is the pool's one modal spell, and checks
the boundary the guard hid (a chosen mode past the end of the list, and the
last real mode in range).

That is the second vacuous test this campaign has found (the first was
`a_first_strike_blocker_kills_before_the_attacker_strikes_back`, whose
assertion held under its own mutant because a dead blocker's damage clears
on the zone change). Both were found by mutation, and neither could have
been found by reading — which is the argument for the instrument.

## The clauses that got tests

| where | what had no bad state behind it | new test |
| --- | --- | --- |
| `transition.rs::action_contract` | a declaration naming an attacker nobody submitted, or one not eligible; a block by a tapped creature, by the attacking player's own creature, or one never submitted; a cast refused that moved the card anyway; an activation that neither reached the stack nor backed out; a concede that does not record the loss; a mid-payment cast that vanishes with the card and no `SpellCast` | `invariant_families.rs`: `a_declaration_that_disagrees_with_the_submitted_attackers_is_flagged`, `…_blocks_is_flagged`, `a_cast_that_neither_resolved_nor_was_cleanly_refused_is_flagged`, `an_activation_that_neither_went_on_the_stack_nor_backed_out_is_flagged`, `a_concede_that_does_not_record_the_loss_is_flagged`, `a_pending_cast_that_vanishes_with_the_card_is_flagged` |
| `stack.rs::check_trigger` / `check_core` | a trigger carrying a target its ability never asked for (and matched by kind, not merely by the card having some targeting ability); an Aura on the stack targeting the wrong kind of thing; a stashed payment naming permanents the caster does not control, cards outside their graveyard, or the same one twice; and the healthy shapes — an instant above the sorcery it answered, an ability that really did sacrifice something, one state trigger in flight | `invariant_families.rs`: `a_trigger_carrying_a_target_its_ability_never_asked_for_is_flagged`, `an_aura_on_the_stack_targeting_the_wrong_kind_of_thing_is_flagged`, `a_stashed_payment_that_names_the_wrong_permanents_is_flagged`, `a_payment_waiting_under_the_wrong_prompt_is_flagged`, `the_healthy_shapes_of_the_stack_are_not_flagged` |
| `events.rs::damage` | a blocker hitting something other than what it blocks; a blocked attacker reaching the player without trample; life loss for the wrong player or of the wrong amount; lifelink with no life gain; a regular striker dealing in the first-strike step | `invariant_event_window.rs::combat_damage_events_agree_with_the_blocks_and_the_step` |
| `events.rs::steps` / `zone_changes` | the cleanup's three marks (damage, who dealt it, deathtouch) each alone; an entry event about a permanent that is not there afterwards; a creature that entered this action and is not summoning sick; a planeswalker's printed loyalty; the zone an object's last announced move put it in; a discard of somebody else's card or of a token; a tap of something that has left | `invariant_event_window.rs`: `an_entry_event_describes_the_permanent_that_arrived`, `a_discard_names_its_players_card_and_a_tap_a_permanent`, and new cases in `a_turn_start_finds_the_board_reset` |
| `legal.rs::activate` / `combat_prompt` | counters an ability cannot remove; equip offered outside a main phase; an artifact ability under Stony Silence; targets for an ability that does not target; a minus ability past the planeswalker's loyalty; and each way an attacker evades a blocker — flying answered by flying or reach, intimidate by an artifact or a shared color, protection, "can't be blocked" | `invariant_legal_offers.rs::an_activation_offer_can_pay_what_the_ability_costs`, and new cases in `the_blockers_prompt_is_the_board_read_back` |
| `prompts.rs::check_choice` / `mulligan_shape` | a trigger-target prompt whose queue front differs by controller or already has its target; the mulligan phase's three conditions each alone; a queued bottoming for a non-player or past a hand; the legend-rule group by name, controller and zone; a damage prompt's options; a library search of somebody else's library; a token attacking its own controller or a creature | `invariant_prompt_shapes.rs`: new cases in `a_trigger_target_prompt_is_for_the_front_of_the_queue`, `the_mulligan_phase_is_turn_one_before_anything_happened`, `the_legend_rule_prompt_is_the_duplicate_group`, and `a_prompts_options_are_ones_its_effect_could_act_on` |
| `objects.rs::check_core` | a modal spell's chosen mode, out of range and in range (the test that claimed this was vacuous — see above); a copy token whose name cache disagrees with its face | `invariant_object_shapes.rs`: the rewritten `a_modal_spell_on_the_stack_chose_one_of_its_modes`, `a_copy_whose_name_cache_disagrees_with_its_face_is_flagged` |

## The second round: judging the backlog instead of bucketing it

The 215 that came out of the first round were sorted into "test this" and
"backlog" by *shape* — a `&&` flip here, a deleted match arm there — and 206
of them went on the backlog on that basis. That was a bucketing, not a
judgment, and it does not survive the guide's two questions. Asked one at a
time — *if this shipped, what would a person see go wrong?* and *would the
test survive a correct rewrite?* — most of them have the same answer, and it
is not "shrug": each names a rule somebody could watch break (a step skipped,
a turn handed to the wrong player, damage that appears from nowhere, a loss
with no reason the state can show), and the test for it asserts the rule, not
the implementation.

So they got states. A fourth cargo-mutants run over all 215, against the
tests those readings produced, **caught 132 and missed 83**.

| where | what had no bad state behind it | new test |
| --- | --- | --- |
| `transition.rs::walk` | every step succession CR 500.1 allows (turn one's skipped draw, an attack nobody declared, two combat damage steps, two cleanups, cleanup to untap across a turn) and the same jumps where the rules forbid them; a turn that starts for the wrong player, at the wrong number, or out of the middle of a turn; the opening hands, which sit outside the turn structure | `invariant_families.rs`: `the_step_and_turn_succession_of_a_transition_is_checked`, `the_mulligan_phases_own_succession_is_checked` |
| `transition.rs::identity` / `monotone` / `per_turn` | an object's card changing without a copy or a zone change; a monotone record going backwards; land drops and spells cast against their events, across a turn boundary too | `invariant_families.rs`: `identity_and_monotone_edges_are_checked_one_at_a_time`, `the_per_turn_records_are_checked_against_the_events` |
| `transition.rs::status_ledgers` | tap and untap as edges with the right verb about the right permanent; marked damage growing by what was dealt and shrinking only through regeneration or cleanup; regeneration tapping and leaving combat; an attack stamp with a declaration behind it | `invariant_families.rs::the_status_ledgers_of_a_permanent_are_checked` |
| `transition.rs::life_and_loss` / `mana_ledger` | a draw off the top of the player's own library; the life chain starting and ending where the player is, with an intermediate 0 still a loss (CR 704.5a is about the state, not the endpoints); each loss reason against what the state shows; mana appearing only through `ManaAdded` | `invariant_families.rs`: `the_life_and_loss_ledger_is_checked`, `the_mana_ledger_is_checked` |
| `transition.rs::action_contract` | a land play that moved from hand to battlefield with its event, each half alone; a cast whose cost left the pool, generic part included, with mana tapped inside the same window counting as paid and mana added for the wrong player or colour not; a mana ability that leaves step, priority and stack alone; the hand-size discard moving exactly the cards it names; a mulligan that shuffles before it draws, moves its count, empties the old hand and draws the new one; a bottoming that bottoms exactly what was asked for | `invariant_families.rs`: `the_costs_an_action_pays_are_checked_against_the_pool`, `the_hand_shaping_actions_move_exactly_what_they_name` |
| `legal.rs` / `prompts.rs` boundaries | the legal side of each boundary the earlier tests only broke: a menace prompt asking for exactly two blockers, a tap plan naming one source once, a land offered from the acting player's own hand, an exile cost asking for an exact number of cards, a spell paused off the stack list but still in the stack zone, mana added in a real amount, and combat damage in a combat damage step with a combat — each half of which alone is the violation | `invariant_prompt_shapes.rs::a_healthy_cast_time_prompt_is_not_flagged`, `invariant_event_window.rs::the_events_of_a_step_are_checked_against_the_step`, and new cases across `invariant_legal_offers.rs` |
| `stack.rs::check_core` | the granted-flashback lookup: Past in Flames grants flashback to every instant and sorcery in a graveyard at once, so Devil's Play's `{X}{R}` grant sits beside a `{1}{R}{R}` one, and reading the wrong grant invents an X nobody announced | `invariant_families.rs::a_granted_flashback_cost_is_read_off_the_grant_that_names_the_spell` |

The last row is the fifth pass, and it is also a correction: the granted-
flashback lookup had been written up as *unreachable* in the accepted list
below, on the argument that no granted flashback cost in this pool has an X
in it. That argument is right about the grant naming the spell being cast
and wrong about the mutants, which read a *different* grant — one that
really can carry an X. The two mutants that read the wrong grant are killed
now; the third, which reads no grant at all, is genuinely unreachable and
stays accepted with the corrected reason.


## Accepted, with reasons

Twenty-one mutants over sixteen normalized lines, each one read against the
source rather than sorted by shape. All of them are in
`reports/mutants-accepted.txt` with their reasons attached; the thirteen
this pass settled first are set out below, and the eight the third round
added are argued in that file.

**`replace + with *` on a "later events" scan** — `events.rs` 374:57 (lifelink),
504:30 (`PlayerLost` → `GameEnded`), 517:39 (`CreatureDied` → `TurnStarted` /
`LeftBattlefield`), 554:41 (`LeftBattlefield` → a token's return), 565:47
(entry → a later untap step), 462:24 (the untap-step window).

`events[i * 1..]` is `events[i..]`: the mutation re-includes the event the
arm is currently examining. Every one of these scans looks for a *different*
kind of event than the arm it sits in — a `PlayerLost` is not a `GameEnded`,
a `CreatureDied` is not a `TurnStarted`, a `LeftBattlefield` is not an
`EnteredBattlefield` — so the extra element can never match and the mutated
program cannot behave differently. Equivalent, in the guide's first accept
bucket.

**`objects.rs` 261:60 `replace < with >`** — `modes.len() < 2` becomes
`modes.len() > 2`. Ghoulcaller's Chant is the pool's only modal spell and it
has exactly two modes, so both predicates are false for every reachable
input. Unreachable with the current pool.

**`stack.rs` 424:78 `replace == with !=`** — the X-funding prompt's "was the
sacrifice already made?" test. Reaching it needs an activated ability with
both an X in its cost and a sacrifice cost; the pool's three sacrifice-cost
abilities (Grimgrin, Skirsdag Cultist, Disciple of Griselbrand) have no X.
Unreachable with the current pool. The `delete !` a few characters away
stays on the backlog instead, because its normalized form would cover every
other `!` in `check_core` too.

**`stack.rs` 106:95, the guard `*target == obj.id` forced to `false`** — the
granted-flashback lookup finds nothing instead of the right grant. The two
answers differ only when a *granted* flashback cost has an X in it: Devil's
Play is the pool's one card whose mana cost has an X, and it prints its own
flashback cost, which is consulted first and short-circuits the lookup
before the grant is ever read. Unreachable with the current pool — and,
unlike the other two mutants of that same guard, not fixable by a test,
which is why they were killed and this one was not.

**`legal.rs` 740:21 `replace - with +` in `choose`** — `k.min(n - k)` becomes
`k.min(n + k)`, which is plain `k` (the function has already returned 0 if
`k > n`). The `min` is the C(n,k) = C(n,n−k) symmetry, taken only so the fold
runs the shorter way round; the binomial it computes is the same number.
Equivalent.

**`legal.rs` 451:27 `replace < with <=`** — `def.loyalty_change < 0` becomes
`<= 0`, admitting a change of exactly 0 to a body whose test is
`0.unsigned_abs() > counters`, i.e. `0 > n`, false for every n. The extra
input reaches the clause and cannot make it speak. Equivalent.

**`transition.rs` 594:48 `replace + with -` in `mana_demanded`** — the term is
`cost.colorless_amount()`, the `{C}` symbol, which post-dates this card pool:
`ManaSymbol::Colorless` appears nowhere under `src/cards`. The term is always
0, and `x + 0` is `x - 0`. Unreachable with the current pool.

**`transition.rs` 853:45 `replace > with >=`** — `s > d` compares the position
of the `LibraryShuffled` event with the position of the first `CardDrawn` in
one and the same event vector. Two events of different kinds cannot occupy
the same index, so `s == d` is impossible and the two comparisons agree on
every input. Equivalent.

## The third round: the backlog, worked

The 68 that came out of the second round were left on the backlog with
the argument that each named a real gap and only the fixture was missing.
That argument was then tested by writing the fixtures. All 68 are
resolved: **60 got a test that kills them, and 8 turned out to be
unreachable** on any state this engine can produce, with the reasons
written into `reports/mutants-accepted.txt`.

The 60 kills came from about twenty new states across the five families,
and the pattern the second round predicted held: nearly every one needed
a state that satisfies part of a clause and not the rest.

| where | what had no state behind it |
| --- | --- |
| `transition.rs::action_contract` | an activation's stash naming a different ability or activator; a refusal that tapped for mana and backed out; an activation cost tapped for inside the same window; a paused resolution that was resumed, that finished, or that came back as a new object; a pending cast replaced by a different one |
| `transition.rs` ledgers | a token that ceased to exist after leaving the battlefield; a verb paired with a move that went somewhere else; a flashback cast out of the hand; a regenerated blocker still in combat; "the top of the library" measured in the cards that left THIS player's library; life that arrives at zero with no `LifeChanged`; a batched turn boundary with no events to read |
| `events.rs::damage` | lifelink gaining the wrong amount; an unblocked attacker hitting the planeswalker it attacks; a blocked trampler hitting a creature that is not blocking it; a creature that left the battlefield with damage still marked; first-strike discipline in a window where something died |
| `events.rs` windows | "a later event about this card" meaning this card; a token that is not exempt from summoning sickness; a cast that raised a prompt instead of handing priority back; a block on an attacker that left; a draw step drawing one card for the active player |
| `legal.rs` | a finished game offering nothing; Stony Silence letting a land tap; an instant offered outside the main phase; a target on the only stack entry; an ability offered on the opponent's permanent; the copy path of a granted ability, including a Grizzly Bears shaped by Essence of the Wild; the ability half of the X-funding stash |
| `prompts.rs` | each of the three trigger queues and each of the three things in flight, one at a time; a YesNo and a PayOrNot naming their source; a debuff and a can't-block prompt; a two-option trigger-target prompt; the active player working their own queue; a library card listed under the wrong owner |
| `stack.rs` | a state trigger in the queue that is for state triggers; the opening-hand loop and a finished game, each exempt from the scan claim on its own |

The 8 that could not be killed are not a shrug either, and they are not
the shape the second round guessed. Every one of them turned out to be
unreachable rather than untested — a clause guarding a state the engine
cannot build (a discard event for another player inside a hand-size
discard, a permanent off the battlefield with no `LeftBattlefield` event,
a card entering the library without its zone-change count moving) or a
card the pool does not have (nothing grants a player protection from a
colour; no ability has both an X and a sacrifice in its cost). The
reasons are in `reports/mutants-accepted.txt`, each one written against
the source rather than the shape.

So the second round's own claim — "these are honest gaps rather than arid
mutants" — was right about 60 of 68 and wrong about 8, and the way to
find out was to write the tests.

**Verified**: a fifth cargo-mutants run over all 68 at the branch head —
`60 caught, 8 missed in 13m` — and the eight it misses are exactly the
eight argued unreachable above, name for name. Each kill was also watched
failing under its own mutant as it was written, which is the check this
campaign has learned not to skip: a mutation-motivated test is not done
until it has been seen killing its mutant.

## The one that got lost

`prompts.rs:514:99: replace && with || in library_option` was filed, missed
the nine-binary pass, and then vanished: the remap that moved eight shifted
mutants onto their current line numbers matched its neighbour at column 75
and dropped it. It is re-tested here — `a_prompts_options_are_ones_its_effect_could_act_on`
kills it — and recorded as `pass4-caught` in the raw log. Mentioned because
the arithmetic in the re-run should close, and until this was chased it was
off by one.
