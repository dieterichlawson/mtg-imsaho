# Formal verification of the engine and its cards

Started 2026-10-25. This is the plan and the running record: what "proved
correct" can mean for this engine, what has been proved so far, what each
proof does and does not cover, what the tools turned out to be able to do,
and what comes next.

## What "correct" means here

A proof shows that code meets a specification. It does not show that the
specification is right. For this engine the specifications are the
Comprehensive Rules (CR) and each card's Oracle text, and neither is formal.
So every piece of this work has two halves:

1. **Write the rule down formally**, in a form a person can check against
   the CR or the Oracle text by reading it. This is the trusted part. A
   wrong spec makes a proof worthless, so a spec is reviewed the way a
   card's acceptance tests are reviewed today: against the printed words.
2. **Prove the code meets it**, by machine, for every input in a stated
   space rather than a sample of them.

What this cannot deliver is one theorem that says "the engine is correct".
The engine is about 50,000 lines of Rust covering about 270 cards, and
Magic's rules are rich enough that the general game is Turing-complete
(Churchill, Biderman and Herrick, "Magic: The Gathering is Turing
Complete", 2019). An end-to-end refinement proof of the whole engine against
a full model of the CR would be a project on the scale of seL4 or CompCert.
What can be built, starting now, is a growing set of proved properties. Each
one rules out a class of bug for good, and the tests, the fuzzer, mutation
testing and the playtests keep covering everything the proofs do not reach
yet.

### What already exists, and what proofs add

| instrument | what it shows |
|---|---|
| acceptance tests (`mtg-engine/tests/cards_*.rs`) | a card does what its text says in the situations someone thought of |
| rule tests (`mtg-engine/tests/*.rs`) | a rule holds in the situations someone thought of |
| invariant oracle (`src/invariants/`) and the nightly fuzz | the properties in `check_core`, `check_settled`, `check_transition` and `check_legal` hold in every state the random seats reached |
| mutation testing (`weekly-mutants.yml`) | where the suite would not notice a change |
| `card_data_invariants.rs` | a card's printed data and Oracle text match Scryfall (`data/oracle_cache.json`) |

All of these find bugs; none can show that a bug is absent. A proof covers
every input in its space, including the ones nobody thought of and the
fuzzer never reached. The first three specs written for this effort found
five defects in code all of the above had passed, and the bounded
exhaustive check of the autotap planner a sixth (see
[defects found](#defects-found)).

## Tools, and what they turned out to be able to do

**Verus** is the main tool. It proves Rust annotated with contracts
(`requires`, `ensures`, loop invariants) using the Z3 SMT solver, which
reasons about integers rather than bits, so a proof holds for every amount.
The proved code lives in the `mtg-kernel` crate, inside `verus!` blocks.
`cargo build` erases the contracts and compiles the rest as ordinary Rust,
so the engine runs the code that was proved, not a copy of it. Only
`mtg-kernel` depends on `vstd`. Its kernels (109 obligations) verify in
about three seconds, once `vstd` is built. Verus has no specification for some of `std` (it rejected
`slice::swap` and `sort_by_key`); the kernels write those few lines out.

**Kani** (bounded model checking of the compiled Rust, through CBMC) proves
what Verus cannot see: the engine-side glue that turns a `ManaCost` into
what the kernel takes (`mtg-engine/src/proofs/`). Measured on this
codebase:

- **Small pure functions over small values: fast.**
- **Sums of many numbers: the cost grows steeply with the values allowed.**
  CBMC reasons bit by bit. The payment reserve proof finished in 40 seconds
  with at most 3 of each type of mana and did not finish in nine minutes
  with at most 5. Verus proves the same property for every amount in about
  a second; that measurement is why payment moved to Verus.
- **`BTreeMap`: out of reach.** One insert and lookup proves in five
  seconds; six inserts did not finish in five minutes. Most of the engine's
  state (`GameState`, `ManaPool`, `CombatState`) is `BTreeMap`s; a Kani
  build keeps the objects in a `VecMap` and the counters and mana pools in
  a `KindMap` instead, each proved an ordered map (below), and each also
  held to `BTreeMap` by unit tests, on random sequences of hundreds of
  operations past the proofs' bounds, so that `cargo test` and the weekly
  mutation sweep see them (`reports/mutation-testing.md`, "The proofs'
  stand-ins"). The card-name `NameList` that stands in for a `HashMap` is
  held to one the same way.
- **`flatten` and `flat_map` asked for their items one at a time: out of
  proportion.** Kani follows `Flatten::next` through every round its loop
  is unwound to, for every item asked for, so a `for` loop, a `zip` or a
  `collect` over one costs what the unwind bound allows, not what the
  input holds. Measured on a crate of the shapes alone, at unwind 9:
  two `Option`s walked by a `for` over `flatten`, 102,000 steps (8,600 at
  unwind 3); the same as an `if let` in a plain loop, 2,200; two sources'
  three abilities collected through `flat_map`, 399,000, against 12,000 for
  `any` over the same `flat_map`; one blocker gathered by
  `chain(.. .flatten()).collect()`, 482,000, against 10,000 for two loops.
  An adapter that drives it by `try_fold` (`any`, `all`, `sum`, `max`,
  `find`, a `filter` after it) is cheap; every other adapter measured
  (`chain`, `zip`, `enumerate`, `rev`, `skip`, `take`, `peekable`,
  `filter`, `map`) stays within five times the plain loop. So code the
  proofs follow walks nested lists with nested loops: the trigger order
  proofs' cases (655,000 steps to 226,000), the first-strike check of
  every combat damage step (`CombatState::any_blocking`), the log line of
  declared blocks and a modal spell's targets.
- **A branch the code never takes, on a test symbolic execution cannot
  fold, is followed all the same, and what it returns spreads.** CBMC
  does not narrow a value by the branch it is on: a `Vec` returned
  `Some` on one path and `None` on another is, after the join, of a
  length it leaves open, and every loop over it runs to its bound even
  where the `Some` is known. `a.min(b)` with `b` zero and `a` the solver's
  is such a test (`min` is an `if`), and so is anything whose answer is
  settled only by arithmetic. Measured on a crate of the shape alone: a
  plan built after a deduction of `min(pool, 0)` and walked, 33,500
  steps, both loops at their bound; with the deduction asked only when
  something is owed, 4,400. The autotap planner deducted the pool's `{C}`
  so for every cost (now only for a cost with a `{C}` pip), and the
  check that its plan pays then ran each step of a one-source plan four
  times.
- **A list of large structs lifted out into `Option`s and put back runs
  the solver out of memory.** Two triggers moved into a
  `Vec<Option<_>>`, taken out in a chosen order and pushed back: out of
  memory in the solver at 7 GB, 130,000 steps. The same reordering by
  swaps in place: 28,000 steps, proved. The ordering answer of
  triggers (`ChooseTriggerOrder`) now swaps.
- **CBMC's SMT backend** (`--z3`) crashes on Kani's output (`map::at`) as
  of Kani 0.68.

The pattern every proof so far follows: pull a rule out of the state into a
function over plain values (arrays, slices, small structs), prove that
function in `mtg-kernel`, and keep the layer that reads the state into it
and writes the result back small, with unit tests on it.

### Running the proofs

    scripts/prove.sh                   # Verus, then every Kani proof: some 85 hours of solver time
    scripts/prove.sh verus             # mtg-kernel only: a minute or two once vstd is built
    scripts/prove.sh kani [NAME...]    # the Kani proofs, or the named ones or modules
    scripts/prove.sh kani-pr           # the pull request's tier: under half an hour
    scripts/prove.sh kani-shard K/N    # the Kth of N shares of every Kani proof
    scripts/prove.sh exhaustive        # the bounded exhaustive checks, in release
    scripts/kernel-mutants.py [ID...]  # what the proofs and the tests each catch
    scripts/fuzz-engine.sh [SECONDS]   # the whole engine under libFuzzer (tier 2)

Every Kani proof is too many to run on each change, so they run in two
tiers. The `proofs` workflow, on every push to master and every pull
request, runs Verus over `mtg-kernel`, the exhaustive checks, and the
pull request's tier of the Kani proofs (`kani-pr`, held to 30 minutes):
`scripts/kani/pr.list`, the fastest proved harness of each proof module
within a budget, and the proved harnesses of each proof module the change
edits, fastest first, within a budget of their own. `proofs-full`, on the
1st and the 15th of each month, runs every Kani proof in 30 jobs, shared
out by how long each harness took last time (`scripts/kani/times.txt`),
each cbmc held to 12 GB, and files an issue naming any harness that
proved last time and does not now. Either tier fails on a harness that
does not prove unless `scripts/kani/expected-open.txt` names it, and says
so of one it names that now proves. `scripts/kani-tiers.py write`, run on
the full run's merged results, refreshes all three lists. A change to the
engine reaches proofs outside the modules it edits, which the pull
request's tier samples rather than runs; the full run is what holds them.

`nightly-fuzz` runs the coverage-guided target for 50 minutes a night.
The Verus release (0.2026.10.04), its Rust toolchain and the Kani version
(0.68.0) are pinned in `scripts/prove.sh`, and `vstd` in
`mtg-kernel/Cargo.toml` must match the Verus release. `kani-bounded.py`
runs one cbmc at a time unless told otherwise (`--jobs`, each held to
`--mem-cap`): four at once ran a 15 GB machine out of memory. It records
each harness's result as it goes and, run again on the same results file,
picks up where it stopped, a harness whose build failed included.

A harness can pass and prove nothing. If an assumption, a stub or an
unwinding bound rules out every execution, no assertion is ever reached,
and Kani reports success with each assertion UNREACHABLE.
`kani-bounded.py` reads the reachability check Kani puts before each
assertion. Each result line counts how many of the proof's own `assert!`s
were reached (`reaches 11/22`), and a harness that reached none is
VACUOUS, not SUCCESSFUL. A harness split into constant cases legitimately
leaves the other cases' assertions unreached, and their covers too.

### What is trusted

A proof is a proof relative to what it assumes. Here that is:

- the **specification functions** (`pub open spec fn` in `mtg-kernel`, and
  the specs in `mtg-engine/src/proofs/`). These are what a person reviews
  against the CR; everything else is checked by machine;
- **Verus, Z3 and Kani/CBMC**, and `vstd`'s specifications of the `std`
  functions the kernels call (`min`, `clamp`, `saturating_sub`, `Vec::push`);
- **rustc**, compiling the same source for the game that Verus checked;
- **a defect in Kani 0.68 that the proofs keep clear of**, and a check
  that they do. Kani shares one global between equal constants through a
  table of the allocations it has laid out, and lays out a static's
  initial value through the same table, under the static's own symbol
  (`codegen_static` calls `codegen_alloc_in_memory`). A constant laid out
  after it with the same bytes is then read from the static, and reads
  whatever the program last stored there. The proofs' stubs talk to
  their harnesses through `static mut`s, and a counter `MARKED_N: usize
  = 0` shares its eight zero bytes with the standard library's
  `ZERO_CAP`, the capacity of every `Vec::new()`: once a stub had counted
  one creature dealing first-strike damage, every new list had a capacity
  of 1 and no buffer, and the first-strike damage proofs failed on a
  write through a dangling pointer. A constant read so is no longer the
  program Rust compiled, in either direction, so a proof's result with
  one is not a result about the engine. Every static of the proofs is
  now a `Held` (`mtg-engine/src/proofs/held.rs`), its value beside its
  own address: an allocation's provenance is part of what the table
  compares, and no constant points at a static of the proofs, so none
  has a `Held`'s bytes. `scripts/static-constants.py` reads each
  harness's goto program for a constant read through any static, or a
  pointer into one of the proofs' statics from code outside the proofs,
  and `kani-bounded.py` records a harness with either as an ERROR rather
  than run it. Before this, one harness from each of the proofs' 104
  modules was scanned: 34 read a constant through a static. In 30 the
  only such read is `raw_vec::new_cap`'s zero capacity for a zero-sized
  element, a branch no list of a sized type takes; in four (an ability's
  X answer, keeping a hand after mulligans, a state trigger's
  resolution, the spells counted across a turn) `Vec::new()` read its
  capacity from one of the proof's counters, and each of the four had
  passed. Every result from before this was produced with the defect
  present, so the full run after it (below) is the record;
- **a known imprecision in CBMC 6.11's `memcpy`**, which Kani 0.68 uses
  for every copy of a list of plain values (a `Vec<ObjectId>`'s clone, a
  `String`'s). It copies the source's bytes as an array taken where the
  pointer points, and where symbolic execution cannot tell which object
  that is, it takes, for a heap buffer, its first element rather than
  the buffer, so every byte but the first is left unconstrained. The
  smallest case is about thirty lines of Rust: a clone of
  `Outer::Choice { player, source, choice: Inner::Narrow { looked_at } }`,
  its list built in place, reads back a second element that may be
  anything, and plain `cargo kani` reports the clone's own contents
  wrong. Harnesses that must fail (that the element is not the one
  copied, that a pushed-to list is not three long) still fail, so the
  copy is an over-approximation: the true value is among the ones
  allowed, and a proof that passes still holds. What it costs is
  proofs: a list of any contents is a list every loop over it walks
  every way, and the proofs of choices (an engine prompt is such an
  enum) ran out of memory on it. A `memcpy` of our own, a word at a time,
  was precise but ran a thirty-line harness's solver out of memory, so
  the engine avoids the copies on the paths the proofs follow instead
  (`resolve_choice`, the damage queue);
- the **glue** that is not proved: reading `ManaPool`'s `BTreeMap` into six
  counts and writing the result back, mapping blockers' ids to positions,
  turning X's source groups into counts and the kernel's taps back into
  mana, reading the board into the turn's four facts, and acting on a
  creature's fate (the matrix of blocks the rules allow pair by pair, read
  off the board by `combat::can_block_attacker`, is proved with Kani:
  `proofs::actions::blocking`). Each is a few lines with unit tests; the
  combat mapping is also covered by the exhaustive division test, and the
  X pool step is asserted against the kernel's proved amounts on every
  call in a debug build.

## The tiers

### Tier 1: pure kernels against rules-derived specs

Pieces of the engine that are arithmetic or combinatorics over a few
values. Each gets a spec written from the CR, proved for every input.

| target | spec (CR) | status |
|---|---|---|
| paying a cost from the pool: `can_pay`, `auto_pay`, `auto_pay_reserving` | 107.4, 601.2h | **proved**, three defects fixed |
| combat damage division, `divide_combat_damage` | 510.1c-d, 702.19b, with 509.2's order | **proved** sound and complete, one defect fixed |
| X funding from floating mana, `allocate_for_x`'s pool step | 107.3, and the reserve property of payment | **proved** by reuse of `pay_around`, one defect fixed |
| X funding: which X can be paid, and the taps that pay it (`fundable_x_values`, `allocate_for_x`) | 107.3, 601.2f-h | **proved** for every board and every X |
| autotap planner soundness: a plan it returns pays the cost | 601.2g-h, 605.3 | **proved for every board**: every plan returned has been run and found to pay, each step of that run is proved to be what the engine's activation does for each kind of source in the pool, and a plan whose side effects change what later steps can do is rehearsed (see [the autotap planner](#the-autotap-planner-every-small-board)); two defects fixed. Also checked exhaustively on every board of up to three sources (29.6 million) |
| autotap planner completeness: if some set of sources can pay, it finds one | 601.2g | **by construction** on every board of 4,096 plans or fewer: where the greedy pass offers nothing that pays, every plan is tried (`searched_plan`), the proved count first; **checked exhaustively** as above, one defect fixed; on a bigger board, the greedy pass alone |
| the auto-pass gate's count (`within_reach`): never rules out a cost some plan pays | 601.2g, 605.3 | **proved** for every pool, every board and every plan (`mtg-kernel/src/reach.rs`), one latent defect fixed; also checked exhaustively on every board of up to three sources |
| power and toughness from counters and effects (layer 7) | 611.3a, 613.1f, 613.4 | **proved** on the engine with Kani for a setting, +1/+1 and -1/-1 counters and a change at once, each change until end of turn, and each change an Aura, Equipment or lord prints (`proofs::actions::power_and_toughness`, `until_end_of_turn`, `effects`, `scopes`), on two creatures; **checked** against the card-semantics reference on every cast, death and combat; not extracted into a kernel |
| lethal damage, deathtouch, zero toughness, and counter pairs | 704.5f-h, 704.5q | **proved** (`sba::creature_fate`, `sba::annihilate`), called by the state-based actions; indestructible, regeneration and attachments stay in the engine, checked against the same reference |
| the turn's step sequence and who gets priority | 500.1, 103.7a, 508.8, 510.4, 514.3a, 117.3-4 | **proved** (`turn::next_step`, `turn::after_pass`), called by `advance_step` and the game loop |
| which submitted blocks stand | 509.1a-b | **proved** (`blocks::judge_blocks`), called by `declare_blockers` |

### Tier 2: the whole engine, through the fuzz oracle

The fuzz oracle's properties (`check_core`, `check_settled`,
`check_transition`, `check_legal`) are already a formal statement of what a
coherent game is, reviewed and maintained. The Anthropic post on LLM-driven
verification (2026-10-22) turned each existing fuzz harness into a spec ("this
test always passes") rather than writing specs by hand. The same move here
gives the whole-engine theorem: **from any state a game can reach, no legal
action makes the engine panic or break an invariant.**

1. **Done**: a coverage-guided fuzz target (`fuzz/fuzz_targets/game.rs`,
   `scripts/fuzz-engine.sh`) to the rules of Theorem's verification skill.
   The input bytes decode, totally, to a game: two pick the coverage decks,
   eight seed the engine, and every choice both random seats make is drawn
   from the rest. Every invariant the seeded campaign checks is checked at
   every decision, plus card conservation, a decision that offers nothing,
   and a save round trip; a violation, a stall or a panic (overflow
   included) aborts. Games are bounded by the input and a decision ceiling,
   and the input is the only randomness. With an invariant deliberately
   broken it reports the break within its first inputs. See
   [the whole engine under libFuzzer](#the-whole-engine-under-libfuzzer)
   for what it has run.
2. That target is the shape Theorem's pipeline proves things about (Rust is
   supported, and they have offered to run it for open-source projects). The
   largest program in their report had about 51,000 lines linked in, so a
   proof of this engine is a stretch; their experience is that attempts
   find bugs before they finish.
3. Meanwhile, extract the bookkeeping each invariant is about (zones and
   library order, the attachment graph, combat's maps) into kernels in
   `mtg-kernel`, the way payment and combat were, and prove the kernels.

Release builds now check for integer overflow (`Cargo.toml`), so an
overflow the fuzz reaches is a panic and a finding rather than a wrapped
number, the Rust form of the skill's rule that a check must not be compiled
out.

### Tier 3: the cards

"Does this card do what its Oracle text says" needs a formal reading of the
Oracle text. The post's closest case to this engine was a Python
interpreter written in Rust, proved with Verus against a formal semantics of
Python, with the semantics checked against CPython on 1.7 million programs.
This engine is an interpreter of the rules, and the same split applies:

1. **A card-spec language.** A small, typed language of effects in which
   each card's Oracle text is transcribed: "deals 3 damage to any target",
   "target creature gets +3/+3 until end of turn", "when this enters,
   create a 1/1 white Spirit creature token with flying". The transcription
   is the reviewed artifact, checked against the Oracle text by reading. It
   is deliberately not the engine's `CardBehavior` code: it is an
   independent statement of what the card means. The post's agents wrote
   specs a hundredth the size of the programs they described; a card's spec
   should be a line or two.
2. **A reference semantics** for that language, written from the CR, in
   Verus spec functions, sharing no code with `src/cards/`.
3. **Refinement**: each card's behaviour agrees with its spec. Most cards
   are thin calls into `cards/helpers.rs` and the damage and zone
   primitives, so the shared helpers are proved once (the post's point
   about dependencies following a power law) and each card's obligation is
   "calls the right helper with the right numbers". Where the engine state
   is in the way, the check is a differential test over the states the
   fuzzer reaches: weaker than a proof, with the same independent oracle.
4. **Whether the semantics is the real game** is checked empirically, as
   the post checked its semantics against CPython: against the acceptance
   tests, and, if it becomes worth it, against XMage or Forge (both
   implement Innistrad) on the same games.

Cheapest first: vanilla and keyword-only creatures (all engine, data
already checked against Scryfall), then the core cards in `src/cards/*.rs`,
then Innistrad by mechanic, in the groups the `cards_*.rs` files use.

The first families are done in a form that comes before steps 2 and 3:
the spec language is a grammar over the Oracle text itself rather than a
transcription, the reference semantics is a test-side interpreter rather
than Verus spec functions, and refinement is checked on fixed boards at
every legal target and every answer to every question rather than
proved. See
[cards against their Oracle text](#cards-against-their-oracle-text).

### Tier 4: whole-game properties

That priority passing ends (CR 117.4), that the stack resolves last in,
first out (CR 405.5), that state-based actions reach a fixed point (CR
704.3). The first plan was a Lean model of the turn and stack machine. The
post argues for proving the code that runs rather than a translation of it
(a model can disagree with the code), and Verus has now proved real engine
code here, so the preference is: extract the turn-step and priority
functions into `mtg-kernel` and prove them there; a Lean model only for what
no extraction can reach, checked against the engine on fuzz traces.

The first part of that is done: the step sequence and what a pass does are
proved in `mtg-kernel/src/turn.rs` and are what the engine runs (see [the
turn and priority](#the-turn-and-priority)). What a round of passes does
follows from them: with no player acting, a round ends after exactly as
many passes as there are players. Termination of a whole turn needs the
number of cleanup repeats bounded, which is a property of the cards, not of
the turn.

## Proved so far

### Mana payment

`mtg-kernel/src/mana.rs`, called by `mtg-engine/src/mana.rs`. The spec: a
coloured pip is paid only with its colour, `{C}` only with colorless,
generic with anything, and `{X}` is zero (CR 107.4, 107.3e, 601.2h). So a
pool can pay a cost exactly when it has enough of each type for that type's
own pips and enough in all (`payable`; Hall's condition for the matching).
Proved for every pool, every cost and every reserve:

- `pay_fixed` (`can_pay`, `auto_pay`) and `pay_around`
  (`auto_pay_reserving`) succeed exactly when the pool can pay, and a
  success spends exactly the cost, each pip from its own type
  (`paid_exactly`);
- `pay_around` leaves the reserve payable whenever the cost and the reserve
  were payable together (`payable_together`, `reserve_payable`): the
  promise of #252, #678 and #840;
- `surplus_order` names every slot exactly once. Which spare goes first is a
  preference the proof leaves open, and a unit test holds it to the
  `sort_by_key` it replaced over 531,441 pools.

Kani proves the engine-side step from `ManaCost` to the kernel's counts
(`each_type_has_its_own_slot`, and `counting_a_cost_counts_every_pip` for
costs of up to four symbols: every printed cost).

### Combat damage division

`mtg-kernel/src/combat.rs`, called by `divide_combat_damage`. The spec,
`legal_division`, from CR 510.1c-d and 702.19b with the damage assignment
order of CR 509.2: every point of damage is assigned (none at all with no
blocker left and no trample); a blocker gets damage only once every blocker
before it in the order has lethal; only a trampler sends any past its
blockers, and only once all of them have lethal. Proved for every power,
every number of blockers and every lethal amount:

- **sound**: whatever the attacking player answers, the division is legal;
- **no empty questions**: every open blocker offers at least two amounts;
- **complete, question by question**: for any legal division, while the
  answers given agree with it, either nothing is left to ask and the result
  is that division, or the next question is about the first blocker still
  open, its range contains that division's amount, and everything before it
  already matches. So every legal division can be reached by answering, and
  no answer already given is ever overridden.

A rules-version note for whoever owns the spec. The engine follows damage
assignment order, which is the rule of Innistrad's time. The current rules
dropped it in the Foundations update (November 2024): the attacking player
now divides damage among blockers freely, and lethal matters only for what
tramples over. The proof is stated against the rule the engine implements;
moving to the current rule would change `legal_division` and the prompt
together.

### X funding

`mtg-kernel/src/funding.rs`, called by `fundable_x_values` and
`allocate_for_x`. The sources that can pay X come in groups: `quanta[j]`
mana for each tap of a group-`j` source, `counts[j]` sources in the group.
The spec, `reachable`, says a sum is tappable for exactly when some number
of each group's sources, none past its count, adds up to it; floating mana
tops a tap sum up by anything from nothing to the whole pool. Proved for
every group, every count, every pool and every X:

- `reachability`'s rows are exactly the reachable sums, including the
  per-residue pass that keeps a group of thousands of lands linear;
- `fundable_amounts[m]` holds exactly when some tap sum the pool can top up
  comes to `m`, so the values the prompt offers are the values that can be
  paid (#595);
- `fund`'s taps are an allocation the board has and add up to its tap sum;
  taps, pool and shortfall add up to what X asks; nothing is short exactly
  when some allocation pays X exactly (#593); an exact allocation taps as
  little as any exact one, so the pool is spent first; and nothing funds
  more of X than this does.

The pool step is `pay_around` with an all-generic cost, so it carries that
proof as well: it leaves what the rest of the hand needs whenever the pool
could pay both. Which mana it spends is the engine's choice; how much is
asserted against the kernel's on every call in a debug build. The page has
its own copy of the allocator, held to the engine's answers by
`x-funding-cases.jsonl` (1,542 checks over 168 boards).

### The turn and priority

`mtg-kernel/src/turn.rs`, called by `advance_step` and the game loop. The
spec is CR 500.1's twelve steps in order, and the four facts that change
what follows a step, each tied to one step: the first turn's skipped draw
step (CR 103.7a), the declare blockers and combat damage steps skipped
when no creature attacked (508.8), a second combat damage step after
first strikers (510.4), and another cleanup step after a player received
priority in one (514.3a). Proved for every step and every combination of
the facts (75 obligations):

- the turn ends exactly after a cleanup step that does not repeat, and a
  step is followed by itself exactly when the rules repeat it;
- otherwise the next step is later in the turn, every step between is one
  the rules skip from there, and every skip they call for is taken: both
  main phases happen every turn. These determine the next step uniquely.

`after_pass` is CR 117.3d and 117.4: the step ends exactly when every
player has passed in succession with the stack empty, the top object
resolves exactly when every player has passed with it non-empty, and
otherwise priority goes to the next player in turn order. A test holds
the engine's numbering of `Step` to the kernel's.

### State-based actions

`mtg-kernel/src/sba.rs`, called by the state-based actions pass. For every
toughness, every amount of damage and either deathtouch flag (78
obligations): toughness 0 or less puts a creature into its owner's
graveyard, which indestructible does not stop (704.5f); otherwise damage at
least its toughness, or any from a deathtouch source, destroys it
(704.5g-h); otherwise nothing. Damage is compared at 64 bits, so no amount
wraps. `annihilate` is 704.5q: the smaller number of +1/+1 and -1/-1
counters is removed from both, so at most one kind is left and the net
change to power and toughness is the same.

### Block declarations

`mtg-kernel/src/blocks.rs`, called by `declare_blockers`. Given which
(blocker, attacker) pairs the rules allow and each attacker's minimum
number of blockers, `judge_blocks` judges every pair a defending player
submits: accepted, a duplicate, a blocker already blocking (509.1a), a
block the rules refuse, or one of too few for its attacker (509.1b, so
menace). Proved for every submission (92 obligations, no assumptions):

- the pairs accepted are a legal declaration: each allowed, no blocker in
  two, every blocked attacker blocked by at least its minimum;
- a pair is refused as illegal only when the rules refuse it, and always
  when they do and its blocker is free;
- a submission that is a legal declaration is accepted whole.

Each of the three kernels rejects each of four one-line breaks
(`reports/verification-mutants.md`).

### The order a zone is listed in

`mtg-kernel/src/zones.rs`, called by `GameState::objects_in_zone`, which
every card that reads a hand, a library, a graveyard or the battlefield
reads it through. Given every object's id in the map's (id) order,
whether each is in the zone (`in_zone_of`, CR 400.3: a library, hand or
graveyard by owner, the battlefield by controller) and, for a
graveyard, its pile, `zone_order` lists the zone. Proved for any number
of objects and any pile, repeated entries and ids of no object
included (17 obligations):

- every object listed is in the zone, and every object in the zone is
  listed;
- each is listed before every one after it, by where its id first
  appears in the pile and, among those the pile does not list, by id; so
  each once, and in the one order there is (CR 404.2: a graveyard's
  order is its pile's).

A lemma covers the boards the card proofs build, whose piles are the
first of the graveyard's cards in id order: those zones are listed in id
order, which is how the card proofs' stand-in for `objects_in_zone`
lists them (CBMC does not get through the kernel itself: a bounded Kani
proof of the same order over eight objects ran out of memory, each
object being written at a position the data decides). The kernel
rejects each of three one-line breaks: a repeated pile entry listed
again, a member the pile lists listed a second time, and an object
listed for having a member's id without being one.

### The autotap planner, every small board

`compute_autotap` is a heuristic: a greedy pass ranked by a priority key,
then a repair search for plans that strand a spell in hand. A proof for
every board is further off, so the promise it makes is checked
exhaustively over a bounded space instead (`autotap_brute_force.rs`, the
small-scope hypothesis): every multiset of the 23 kinds of source the card
pool has (basics, check lands, Sol Ring, mana creatures, Shimmering
Grotto's filters, Deranged Assistant's side effect), every floating pool
and every cost of each shape a generator draws. On each board the planner
must offer a plan exactly when some way of tapping by hand pays, and the
plan must execute the way the engine executes it, name each source once and
tap nothing it did not need; and `within_reach`, the auto-pass gate's
test, must not rule out a cost some plan pays.

- `cargo test`: every board of up to two sources and one floating mana,
  about 1.4 million.
- `scripts/prove.sh exhaustive` (and the `proofs` workflow): up to three
  sources and two floating mana, 29.6 million boards, under two minutes in
  release.
- Run once: up to four sources and one floating mana for the shapes the
  pool's cards have, 24.7 million boards.

The three-source scope found defect 6 below on 9,500 boards, all with
three-colour costs, which no card in the pool has yet; the random sweeps of
2,000 boards each had not. The four-source scope found nothing for the
shapes the pool has.

Soundness, that a plan offered is one the payment can make, now holds on
every board, by construction and a proof of each step:

1. **The planner returns only plans it has run.** Every plan
   `compute_autotap_around` returns has been through `pool_after`, the
   planner's own run of a plan the way the engine executes one, and found
   to pay. With nothing else to keep payable it asks `plan_pays`, the same
   steps and then `can_pay`, which is `pool_after`'s answer for any
   reserve because `pay_around` pays exactly when the pool can
   (`mtg-kernel`). A greedy plan that does not pay is searched around, with
   something to keep payable, or is no plan. Before, a greedy plan
   `pool_after` found not to pay was handed on untouched, which is how
   issue #252 reached a player (defect 24 below).
2. **Each step of that run is what the engine's activation does.**
   `mana::tap_step` pays an ability's own mana cost, if it has one, around
   the cost being paid and adds what it produces; `proofs::actions::
   mana_abilities` proves that `engine::activate_mana_source_reserving`,
   on the ability as the planner is given it (`available_mana_abilities`),
   leaves the pool exactly as the step does, taps the source and
   announces both, and changes nothing else, for each kind of source in
   the pool: a basic land, a dual's second colour, Sol Ring, a utility
   land, Shimmering Grotto's `{C}` and its filter paid out of the pool
   around the cost, a mana creature, Deranged Assistant; and that a
   source the planner could not run (tapped, summoning sick, a filter
   with nothing to pay for it) changes nothing. Which abilities each card
   has is the card proofs' (`proofs::cards::mana`).
3. **The last payment's success is the pool's.** The engine pays the cost
   around what everything else needs, the planner around the hand; both
   succeed exactly when the pool can pay (`pay_around`).
4. **What the planner cannot see is rehearsed.** A step can change what a
   later one can do only through what an ability does besides adding mana:
   Deranged Assistant's cost mills a card, and with one card left a second
   Assistant's cannot be paid (defect 23). Every offer of a payment plans
   through `engine::plan_payment`, which runs a plan tapping such a source
   on a copy of the game first, as a cast's tap plan always was, and
   offers none that does not pay there.

Completeness holds by construction on every board of 4,096 plans or
fewer (seven sources of two abilities each, or twelve of one): where the
greedy pass and its repair offer no plan that pays, every plan of the
board is tried, the best that pays taken (fewest sources with a side
effect, then the lowest tiers, then the fewest taps) and repaired as a
greedy plan would be. The count asks first, so a cost out of reach,
which no plan pays (`reach_is_necessary`), is never searched. On a
bigger board it is the greedy pass's alone, which the exhaustive check
holds to tapping by hand on boards of up to three sources: a plan it
misses is a missing row on the menu, not a refused payment.

### The auto-pass gate's count

Before passing a seat by, the auto-pass gate asks whether a spell's cost
is within reach of the pool and the sources by counting alone
(`mana::within_reach`): the pool and each source's best net (what its
best ability adds less what it costs) cover the whole cost, and the pool
and the most of each type any one ability of each source adds cover that
type's pips. It may say yes where no plan pays; it must never say no
where one does, or the seat is passed by a spell it could have cast.
`mtg-kernel/src/reach.rs` states the count, computes it (proved to
compute what it states), and proves `reach_is_necessary`: for every
pool, every board of sources and every plan that taps no source twice
and pays each ability's own cost as a payment may (`paid_exactly`), a
cost the plan pays is within reach. The engine's `within_reach` hands it
each ability's cost and what it adds by type. Counting by type is defect
25 below.

### Cards against their Oracle text

`mtg-engine/tests/card_semantics.rs`. A grammar reads each card's Oracle
text, as Scryfall prints it, into effects; a reference semantics written
from the CR, sharing no code with `src/cards/`, says what those effects do
to a board described by its cards' Oracle data, state-based actions
included; and the engine is held to it on a board of fifteen permanents
and six graveyard cards chosen for their differences, and on variants of
it: after a death, with a second permanent of a name, with one of the
opponent's creatures tapped, beside a planeswalker on each side, and,
for a card that targets or reads whose a creature is, with a creature on
each side the other player's card. A battlefield read by owner rather
than controller is caught there in 22 of the 5,269 casts, and nowhere
else.

- **Instants and sorceries**: damage to a target or to each of a group;
  pumps of a target or a group (subtype-qualified too); destroy, destroy
  all, exile, bounce, put on top, tap and untap, "up to N" targets;
  +1/+1 counters; fights, with lifelink; draw, gain and lose life, mill;
  tokens, also "for each creature card in your graveyard"; graveyard
  targets and reanimation; counterspells, against a spell the opponent
  has cast; morbid, on a second board where a creature has died; a cost
  reduction; additional costs (sacrificing a creature, exiling creature
  cards from the graveyard), every way they can be paid, and no cast at
  all when they cannot. Each is cast at every set of targets the engine offers, and
  the engine must offer exactly the sets the text allows and leave every
  zone and every permanent's characteristics where the reference says.
- **Creatures**: keywords, entering tapped or with counters, power and
  toughness defined by a count, and enters and dies triggers whose effect
  is one of the spell templates, with "you may", intervening "if"s and
  "another". A trigger asks its own questions, so the engine is replayed
  every way they can be answered, and the set of games that can result
  must be the set the text allows. Flash is held to the timing rule.
- **Double-faced cards**, both faces, each read with its back face and
  coloured by its colour indicator (CR 204.1, 712.8e): werewolves
  transformed at an upkeep after a turn with no spells and back after a
  turn of two, alongside the two already on the board; any other
  "transform this creature", which does nothing if the permanent has
  transformed since the ability went on the stack (701.28f); and every
  activated ability of a double-faced permanent again with the permanent
  transformed in response, as by Moonmist, where it must resolve as the
  face it was activated from (113.7a).
- **Static abilities**, applied continuously: an Aura's, an Equipment's
  and an anthem's change to the creature it is on or to a group, a lord's
  ("other Spirit creatures you control get +1/+0", "creatures with flying
  your opponents control get -1/-0"), a count ("+1/+1 for each creature
  your opponents control") and a condition ("as long as you control a
  Human", "as long as equipped creature is a Human"). The reference
  applies every static on the battlefield afresh whenever it looks at the
  board and before each pass of state-based actions, keywords before power
  and toughness (CR 611.3a, 613.1f, 613.4c), so a bonus ends when its
  source leaves; an Aura attached to nothing goes to its owner's graveyard
  and an Equipment comes off and stays (704.5m-n).
- **Lands**, and **mana abilities** on any permanent: entering tapped or
  not, the mana each tap makes, a mana ability's own cost, and a creature
  under its controller's control since the turn began only.
- **Activated abilities** on creatures, lands and artifacts, paid with
  mana, {T} and sacrificing the source, at every target the text allows,
  with "activate only once each turn"; and the timing rule for each: with
  the opponent's spell on the stack, an ability activated only as a
  sorcery is not offered and any other is (CR 602.5d); "activate only if
  you control five or more Vampires", on the board, one short and with
  exactly enough (602.5b). **Equip** is one of them, read as CR 702.6a
  defines it, onto every creature the caster controls. And every
  permanent read, either face up, with mana for anything and what either
  face's abilities ask its controller to control, offers no ability its
  text does not print. Each ability is tried again with its source the
  other player's card, as after Traitorous Blood: its "you" is the
  player who activated it (CR 602.2a), and what it sacrifices or returns
  goes to its owner (108.3, 404.2). A controller read as the owner is
  caught in 54 of the 130 abilities. And each, but one its source paid
  for by leaving, is played again with its source exiled in response: it
  resolves all the same (CR 113.7a), from the source as it last existed
  (608.2h), its answers chosen among what is left; what it does to the
  source does nothing, an exchange with the source's toughness exchanges
  nothing (701.12a), and "for as long as you control" a source gone is
  already over (611.2b). The reference was wrong twice before the engine
  was: Tree of Redemption's exchange read a toughness that was gone, and
  Olivia Voldaren's steal outlasted Olivia.
- **Planeswalkers**, Garruk Relentless both faces and Liliana of the
  Veil: each loyalty ability on the board at the printed loyalty, one
  more, one short of its cost and exactly enough. Each face has its
  abilities in the order printed, at the cost printed (CR 606.4), offered
  at exactly the targets its text allows and only when the loyalty pays
  for it (606.6), never with a spell on the stack and never twice in a
  turn (606.3). It resolves every way its text allows: the cost paid as
  it is activated, a planeswalker left with none going to its owner's
  graveyard before the ability resolves (704.5i), damage dealt to it
  removing loyalty (120.3c), "When Garruk has two or fewer loyalty
  counters on him, transform him" turning him over (603.8), each player
  discarding a card of their choice (101.4), a sacrifice of the target
  player's choice, a search after "if you do" only when a creature was
  sacrificed (701.19b), +X/+X for the creature cards in the graveyard as
  it resolves (608.2h), and Liliana's piles: every division, either pile,
  and one division played with each pile chosen, since as a set of
  endings every pile is reachable whoever chooses it. Each ability is
  played again with its planeswalker exiled in response: it resolves all
  the same (CR 113.7a), Garruk's 3 damage dealt and the creature's damage
  back to him dealt to nothing, its lifelink gaining nothing (120.1).
  And each loyalty is tried with the planeswalker its controller's own
  card and the other player's: its "you" is its controller (CR 606.3),
  and one left with no loyalty goes to its owner's graveyard. A
  planeswalker's printed loyalty is now in the Oracle cache too, and
  `card_characteristics` holds `starting_loyalty` to it. And every card
  read, and every activated ability that may target one, is cast beside
  Garruk on the caster's side and Liliana on the opponent's: "any target"
  and "target player or planeswalker" offer them (CR 115.4), "target
  creature" and "each creature" leave them alone, and damage to one
  removes loyalty (120.3c).
- **Triggers on other deaths** ("whenever another creature you control
  dies"): once such a creature has entered, every creature on the board
  dies in turn and every two at once, it among them or not. The reference
  fires the trigger for each death the text names, judged by the dying
  creature as it last was, the watcher seeing deaths it dies with (CR
  603.6c, 603.10a), and puts simultaneous triggers on the stack in every
  order the engine may ask for (603.3b).
- **Upkeep triggers**, on creatures and on Curses, and Auras that keep a
  creature tapped: from the end step of the turn before each player's
  turn, with every land tapped, into that turn's main phase: damage gone,
  only the active player's permanents untapped, the trigger resolved every
  way in the upkeep its text names and no other, and the draw (CR 514.2,
  502.3, 503.1a, 504.1). Curse of Oblivion is played again with a third
  card in the cursed player's graveyard, so that which two it exiles is
  theirs to choose.
- **End-step triggers**, either face up, with intervening "if"s (morbid,
  "didn't attack this turn"): from each player's postcombat main phase,
  on the board and after a death, through the end step and cleanup into
  the next turn's main phase, with the face up by then (513.1a, 603.4).
  Both turn tests also count what the engine puts on the stack against
  the triggers the text fires, so one that triggers when its "if" is false
  and then does nothing is seen. And both play each permanent again as
  its controller's but the other player's card, as after Olivia Voldaren
  took it: "your upkeep" and "your end step" are its controller's (CR
  109.5). A collector reading the owner is caught in 23 of 72 upkeeps
  and 12 of 40 end steps.
- **"You may"**: besides every ending reached, the death checks play
  every "may" taken and every one declined, since as a set of endings a
  wrong answer to one can hide behind a right answer to another. A "may"
  with a cost ("you may pay {1}. If you do, draw a card") is taken only
  when the mana floating pays it, and played with none left.
- **Who answers**: as a set, the endings hide who chose; every way of
  answering is reached whoever is asked. So each question the engine
  asks on a card's behalf, in every test that casts, activates or
  triggers it (spells, activated, loyalty and granted abilities, flashback,
  enter, dies, upkeep, end-step, death, cast and enter watchers, and every
  combat), is held
  to the player its text has answer it:
  its controller, "you" (CR 608.2d); the target player who discards,
  sacrifices or exiles from their graveyard; each player; the countered
  spell's controller who may pay (118.12); the destroyed land's
  controller who may search; Liliana's controller who divides and the
  target player who picks the pile. A combat's damage is the attacking
  player's to order and assign (509.2, 510.1), triggers that trigger
  together are ordered by their controller, the active player's first
  (101.4, 603.3b), and which of two replacements applies to damage is
  the affected player's (616.1), wherever they are asked; and what each
  player chooses, the active player chooses first (101.4). A
  survey of the 172,000 questions the suite asks found none put to the
  wrong player, and two never asked, now asked: Curse of Oblivion's
  choice and Frightful Delusion's discard, each forced by a board that
  held exactly what was owed. The check keeps it so: eighteen mutants,
  each moving one question or one "you" to the other player, or asking
  the other player first, all caught.
- **Triggers on spells cast and creatures entering**: with the watcher on
  the board, its controller casts an instant from hand and by flashback
  and a creature, and the opponent casts in the game loop, so that what a
  cast sets off is collected; a creature entering is judged as it is on
  the battlefield (CR 601.2i, 603.6a). Each watcher is tried again as
  its controller's but the other player's card: "you cast" and "you
  control" are its controller (109.5).
- **Flashback**: every spell that has it cast from its owner's graveyard
  for its flashback cost, at exactly the targets the text allows, the
  card on the stack and not among them, resolving as from a hand and
  exiled after; a sorcery's flashback not offered with a spell on the
  stack (CR 702.34a, 307.1).
- **Abilities an Aura grants**, activated on the creature it enchants as
  the Aura's, and gone with it (CR 113.2c).
- **Declarations, everywhere**: every permanent observed says whether it
  may block at all and whether it must attack, as the engine's own
  declaration checks answer, so "can't block this turn" and "attacks each
  combat if able" are compared in every ending of every check; and every
  creature that does not fight in the combat tests is still offered as an
  attacker and a blocker exactly as its text says (CR 508.1, 509.1).
- **The board's hands and libraries** hold what a choice or a look at
  them can tell apart: a creature card in the caster's hand, a land in
  each hand so that either player is asked in their own main phase, and
  in each library an instant on top and a creature card at the bottom.
  A discard of one's choice, a pick from cards looked at, a reveal that
  transforms, a mill whose mana value counts, are each read and played
  every way.
- **What lasts the turn**: every spell and ability whose effect is
  damage, a pump, a tap or a shield is played on into the opponent's main
  phase, where "until end of turn" changes, damage and regeneration
  shields are gone and mana has emptied (514.2, 500.4).
- **X**: an {X} spell or ability at every X its floating mana allows, and
  Harvest Pyre's "exile X cards" at every subset of the graveyard.
- **Costs** of mana, {T}, sacrificing the source, sacrificing a creature
  (one offer per target and sacrifice, the target among them, 601.2b-h)
  or exiling a card from the graveyard; and each player's floating mana is
  observed, so a cost paid short shows.
- **What resolution asks**: a choice another player makes ("sacrifices a
  creature of their choice", "each player chooses a creature"), a
  search of a library for a basic land card, with failing to find
  (701.19b), and anything done at random, against 32 fixed seeds whose
  union must be every way the text allows (701.9b). A shuffled library's
  order is not compared.
- **Regeneration**: a shield, then lethal damage and, separately, an
  effect that destroys, each of which it must stop (701.19c).
- **Combat**: every creature whose text is keywords, combat text and
  statics attacks, unblocked, with the defender declining to block, and
  against each of the others as a lone blocker, under a reference model of
  CR 508-511: vigilance, defender, haste and summoning sickness, flying and
  reach, intimidate, menace, first strike and double strike, deathtouch,
  lifelink, trample with every damage split the attacker may choose,
  "can't block", "can't be blocked [by <types>]", protection from creature
  types, "attacks each combat if able", conditional haste, tokens that
  enter attacking and are exiled at end of combat, block triggers, and
  triggers on combat damage to a player, the attacker's own and those of
  other permanents. Then the same fights again wherever a permanent
  standing by changes or watches a combatant: a lord, a creature a static
  asks for, an Equipment or an Aura on the combatant, a Curse on its
  player. About 3,700 and 26,000 combats agree; `scripts/prove.sh
  exhaustive` fights every Equipment and Aura on every creature against
  every other.
- **Double blocks**: every kind of fighter attacks again, blocked by two
  creatures at once: every two of a vanilla creature, one with first
  strike, deathtouch, flying and reach, Ashmouth Hound and Abattoir
  Ghoul, each also with a second of its name; and Creepy Doll beside
  the Ghoul. The reference announces each damage assignment order the
  attacking player may (CR 509.2) and divides the damage every way it
  may under it: all of it assigned, each blocker lethal before the next
  is assigned any, a trampler's excess to the player only once both have
  lethal, 1 lethal from deathtouch (510.1c-d, 702.19b-c, 702.2c); both
  blockers deal theirs, a first striker among them before the rest, and
  the order stands for the second step, so a blocker gone after the
  first leaves the other to take it all (510.4); a creature blocked by
  two has each of its block triggers once for each. 1,092 double blocks
  agree, and 1,622 more with each Equipment and Aura on the attacker or
  on one of its blockers wherever it changes the fight (8,320 with every
  fight asked for); `scripts/prove.sh exhaustive` blocks every fighter with every
  two kinds of fighter, about 100,000, which is how the reference was
  found missing Abattoir Ghoul's gain when Creepy Doll's coin destroys a
  creature the Ghoul damaged (its "dies" is any death, not only one to
  damage). Caught on purpose: the reference with the lethal-first rule
  dropped (204 of 587 disagree) or one order only (320), the engine
  placing the blockers in the order declared rather than the order
  chosen (320), and the reference without the Ghoul's gain on a
  destruction (9 of 1,092).
- **Attacks on a planeswalker**: every kind of fighter attacks each
  planeswalker face in the pool instead of its controller, at its printed
  loyalty, unblocked and against each probe as a lone blocker (1,110
  combats). The engine must offer it as what may be attacked (CR 508.1a);
  combat damage to it removes loyalty (120.3c), a trampler's excess over
  its blocker reaches it (702.19c), nothing it is dealt counts as damage
  to its controller, a planeswalker left with none goes to the graveyard
  (704.5i), Garruk transforms at two or fewer (603.8), and a token that
  enters attacking attacks the player or the planeswalker, as its
  controller chooses (508.4b). This found defect 30: Creepy Doll flipped
  its coin for, and could destroy, a Garruk it damaged.
- **Two attackers**: every two of a probe of each keyword and each
  fighter whose combat text or statics act on combat (Rakish Heir,
  Undead Alchemist, Wildblood Pack and the rest) attack together, both
  unblocked, one blocked by a vanilla creature, a first striker or a
  deathtoucher, and each blocked by one of its own (2,895 combats;
  `scripts/prove.sh exhaustive` takes every two kinds of fighter, about
  8,000). All their damage is dealt at once in each step, one
  first-strike step shared (CR 510.2, 510.4), each attacker divided
  among its own blockers, and what the damage sets off is seen as it is
  dealt: a watcher the state-based actions take after the step still
  sees it (603.2, 603.10). The reference was wrong twice before the
  engine was: it looked for Rakish Heir after the actions had taken it,
  and played the end of combat after a lethal attack had ended the game
  (104.2a). Then the engine: defect 31, Undead Alchemist dying beside the
  mill its own replacement made, and the milled creature card left where
  it fell; and, with the reference reading what damage sets off as it is
  dealt, defect 32 in the Aura fights: Curiosity's creature dying of the
  step in which it damaged an opponent drew nothing.
- **Borrowed attackers**: every kind of fighter attacks its own owner
  under the other player's control, as after Traitorous Blood, unblocked
  and against each probe (802 combats). Its damage goes to its owner,
  what it does for "you" it does for its controller, and what dies goes
  to its owner's graveyard (CR 108.3, 109.4, 404.2). A controller read
  as the owner is caught in 70 of them, and a combat damage trigger
  given to the owner in 9. That one went uncaught at first, because no
  trigger hook read its trigger's controller: defect 34. Lifelink paying
  the owner is not caught, the fighters' one lifelinker being a 0/1.

All 268 of the pool's instants, sorceries, creatures, enchantments,
lands and artifacts are read, and its two planeswalkers, and all agree.
Among the latest read is
Frightful Delusion's "unless its controller pays {1}": the spell's
controller paying, by tapping their Forest, and Giant Growth resolving
after; declining, and Giant Growth countered; and the discard either
way. Every template has been shown to catch a deliberate bug in a card
that uses it, and the engine code under it, from Lightning Bolt dealing
2 to the untap step untapping every player's permanents
(`reports/verification-mutants.md` lists the rounds: of the latest 241
mutants, 235 caught, one missed for want of a change of control between
activation and resolution, one for want of a lifelinker with power, one
for want of a spell that reads its target's controller, three
equivalent), and the failure names the
cast and the difference. Every mutant that first survived became a check
or a board: the "may" policies, the stack count, "nothing beyond its
text", the opponent's Vampire, the declarations test, the opponent
casting in the game loop, a Bears cast with no mana left. Inquisitor's
Flail's replacements are read: combat damage dealt by the equipped
creature and to it by another each doubled as it is dealt, after any
prevention (CR 614.1a, 616.1). Skirsdag High Priest's cost of tapping
two other creatures is read, each pair of the caster's untapped
creatures a way to pay it, and its morbid restriction on the board and
after a death. Heartless Summoning's cost reduction is read and checked
on its own: each of five creature cards offered with its cost less {2}
of generic floating and not with one mana fewer, the opponent's at full
price (CR 601.2f, 118.7d). Witchbane Orb's "You have hexproof" is read
and checked on its own: every spell read that targets a player is
offered without the Orb's controller when the Orb is the opponent's, and
with them when it is the caster's own (702.11d); and its enters trigger
destroys the opponent's Curse on its caster. Laboratory Maniac's
replacement is checked by playing the game out: its controller, drawing
from an empty library, wins; without it, or with the opponent's, they
lose for the draw (CR 614.1a, 104.2b, 704.5b). Parallel Lives doubles
the tokens of every untargeted token spell read (Army of the Damned's
thirteen Zombies become twenty-six) on its controller's side and of none
on the opponent's. Stony Silence, on either side, leaves every artifact
read offering no activation, mana abilities included, and Sol Ring
paying for nothing (CR 602.5, 605.1a); with Stony Silence gone from the
engine's every check, 34 of the 51 boards fail. Rooftop Storm's {0} for
Zombies is offered for each of its controller's Zombie creature cards
with nothing floating, and for no other creature card and none of the
opponent's (CR 118.9, 601.2b). Nevermore offers every nonland card's
name in the pool and nothing else, and with Grizzly Bears named neither
player may cast it while Lightning Bolt still may be (201.3, 101.2).
Spare from Evil's protection from non-Human creatures reaches the combat
after it: the caster's Grizzly Bears may be blocked by the opponent's
Human alone, and by all five that may block without it (702.16f). Runic
Repetition is read against a board with three cards in exile, and may
target the caster's Think Twice alone, not their Walking Corpse, which
has no flashback, nor the opponent's Think Twice (406, 115.1).
Ghoulcaller's Chant is the first modal spell read (700.2): its modes are
told by their counts, as the engine's question tells them, and on a
board with a second Zombie card in the caster's graveyard it is offered
at each creature card alone and at the two Zombie cards together, never
one card twice (601.2c). Memory's Journey's targets are read in the
engine's two parts, a player named in the cast and up to three of that
player's graveyard cards asked after it, and its shuffle as random: from
hand and by flashback, each set is offered and resolves as the text says
(701.20a). After Past in Flames resolves, Lightning Bolt and Think Twice
in the caster's graveyard are offered by flashback for their mana costs,
Think Twice for its own as well, and Past in Flames, put there as it
resolves, for its own alone (702.34a, 611.2c); Snapcaster Mage's trigger
is offered those two cards, and gives the one chosen flashback alone.
Dearly Departed, in its owner's graveyard, gives their Humans a +1/+1
counter as they enter, and not Grizzly Bears, nor the other player's
Humans (614.1c, 614.12). Unbreathing Horde enters with a counter for
each Zombie card in its caster's graveyard and none for the opponent's
two Zombies on the board, the count defect 9 got wrong; and Lightning
Bolt or Brimstone Volley at it removes a counter instead of dealing
damage, the last leaving a 0/0 to die (615.1, 704.5f). Mikaeus, the
Lunarch is the first {X} creature read, entering with each X from none,
a 0/0 that dies, to three, and the first cost of removing a counter: its
second ability takes one from Mikaeus and puts one on each other
creature its controller has. Copies are read where they are tokens and
where a creature enters as one (707.2): Cackling Counterpart's token
takes a creature's copiable values and none of its state, and with
Essence of the Wild on its side every creature entering there, Grizzly
Bears cast or Geist of Saint Traft's Angel in combat, is a 6/6 Avatar.
Evil Twin enters as a copy of each of the eleven creatures on the board,
named as it and untapped, or declines and dies a 0/0 (614.12b). Blazing
Torch's creature can't be blocked by Vampires or Zombies in every
Equipment fight, a plain Vampire and Zombie now among the default
opponents, and the ability it grants sacrifices the Torch, not the
creature (509.1b, 701.21a). Undead Alchemist's Zombies make the player
they would deal combat damage to mill that many instead, each creature
card milled exiled for a 2/2 Zombie; with Inquisitor's Flail's doubling
or Ghostly Possession's prevention also applying, the player's choice of
which applies first is played both ways (616.1); and a card milled
counts in its graveyard, as Runechanter's Pike's bonus reads it. Creepy
Doll's flip is played over 32 seeds in each fight it is in, the creature
it damages destroyed in some endings and not in others, as the reference
forks each flip (705.2). Fiend Hunter is the first link between
abilities read (607.2a): it enters twelve ways, each of the eleven other
creatures exiled or none, and a death is checked from each way, the
exiled card returning under its owner's control as a new object. Skaab
Ruinator is cast from its graveyard for its printed cost (601.3a),
exiling three of the four other creature cards there every way and never
itself, not with a spell on the stack, and Skaab Goliath beside it, with
no such permission, not at all. Back from the Brink is checked beside
each of the 128 creature cards the grammar reads put into its
controller's graveyard: offered once for each name of creature card
there and none of the opponent's (602.2b), not with a spell on the stack
(602.5d), and activated with only the card's mana cost floating, X being
0 (107.3), the card exiled and a token entering as a copy of it would,
by the code that enters a cast creature, its enters trigger, counters
and choices all (707.2, 603.6a). Its first run found defect 26: the copy
of Evil Twin was never asked what to enter as. Six mutants (no card
exiled, the token named, the opponent's graveyard offered, the token
entering bare, X of one, not only as a sorcery) each fail. Gutter Grime is the first
enchantment read that watches deaths: on the board beside a Spirit token
of its controller's, each of the twelve creatures there dies, and each
two at once, and each nontoken creature of its controller's that does
puts a slime counter on it and makes an Ooze whose size is the count of
those counters, read each time (604.3), so that two deaths at once make
two 2/2s. Five mutants (tokens counted, the Ooze unsized or sized as it
was made, anyone's creatures counted, no counter put) each fail.
Abattoir Ghoul fights every kind: each creature it deals combat damage
to that dies while it is there, or as it dies too (603.10a), gains its
controller that creature's toughness as it last was (608.2h); its
every-fight run found defect 27, three combats of 621,526 under Wreath of
Geists where the engine read that toughness after the Ghoul itself had
reached the graveyard. Four
mutants (nothing gained, the printed toughness, the dead creature's
controller gaining, any creature's damage counted) fail 96, 1, 95 and
5,355 of the 13,556 default combats; a fifth, the gain resolved after a
player has lost, survives, since no fight kills the Ghoul's creature in
the step a player loses. Grimgrin is the first attack trigger read with
a target, resolved before blockers are declared (508.2, 509.1): it
attacks a defender with no creature, a Grizzly Bears, a Bears and a
Walking Corpse, each the target in turn, and an Invisible Stalker,
which hexproof keeps from being targeted (702.11b, now in the
reference's targeting), so the trigger is removed and its counter not
put (603.3d). Its sacrifice of another creature to untap and grow is the
activated test's, its entering tapped the cast's, and its staying tapped
the turn's. Eight mutants (no counter, the counter with no target,
hexproof ignored, its own creatures targeted, no destruction, the
sacrifice's counter dropped, entering untapped, untapping) each fail.
Bitterheart Witch dies with a Curse of Death's Hold and a Curse of the
Pierced Heart in its controller's library: the trigger targets either
player (603.3d), and its "may" taken, each Curse is found or none
(701.19b), put onto the battlefield attached to that player, and Death's
Hold shrinks that player's creatures at once; the library is shuffled
either way, and not at all if the "may" is declined. Five mutants (the
Curse on its controller, the search bound to find, no shuffle, the "may"
read as a must, the Curse doing nothing) each fail. Mirror-Mad
Phantasm's ability is played with two cards left in its owner's library,
over 32 shuffles: it lands in each of the three places, the cards above
it put into the graveyard and it put back onto the battlefield, a new
object (701.20a, 400.7), and no other ending; four mutants (the revealed
cards kept, the Phantasm not returned, always on top, returned tapped)
each fail. Grimoire of the Dead, the last card read, is the activated
test's: its discard paid with each card of the hand, its study counter
put; its other ability offered with three study counters and not with
two, and every creature card of every graveyard returned under its
controller as a black Zombie (110.2a, 613.1d-e). Seven mutants (not
black, not Zombies, only its controller's graveyard, under their owners,
no discard, two counters enough, no study counter) each fail; with an
Evil Twin card among those it returns, the Twin enters as itself or a
copy of a creature already there, and a black Zombie either way, which
found defects 28 and 29. The card proofs' recorder of a move under a
player's control reports every card moved at once; one Grimoire proof
has it report a card that chooses what it enters as not moved
(`deferring_entries`), and proves that card's entry keeps the creature
there before the return as all it may copy and black and Zombie to be
given (166 s). The entering-as-copy proofs prove the question asked
among what was kept and the answer clearing it; that the grant is then
given is the regression test's. Moldgraf
Monstrosity's branch is the tests' alone, its card proofs being out of
memory already. Hexproof granted until end of turn (Ranger's
Guile on the caster's Grizzly Bears) keeps each targeted instant the
grammar reads, cast by the opponent, off the Bears, and the caster's own
on (702.11b): the reference's targeting ignoring hexproof fails 6 of
those 22 casts. Each is a
template, or a board, to add; `READ` lists the cards read, and a change
that stops reading any of them fails and names it.

### Hidden information

A player is shown nothing of what they cannot see: a library's order
from everyone (CR 401.2), a hand from all but its owner (402.3), what a
player looks at or searches through from the other (701.16, 701.19). So
a player's view, as every seat is sent it, is a function of what that
player may see alone, and changing anything else must leave it the same:
a noninterference property, checked two ways.

- `mtg-engine/tests/view_hidden_information.rs`, on boards of its own:
  each library reordered; the other player's hand and library holding
  other cards, double-faced ones among them; the game played on through
  the other player's draw; and the other player asked to choose a card
  of their hand to discard, what to do with the top card they looked at,
  and what a search of their library finds; and what the acting player
  is offered, with mana for their hand, under the same changes. Each view
  and menu is compared whole, serialized.
- The fuzz runner, at every decision of every invariant-checked game:
  each player's view against the same view with every library reversed,
  and, with no resolution's question pending (which may reveal a hand to
  the player it asks), with the other player's hand dealt anew among its
  objects. 553 games, 88,732 decisions, with the first; 553
  more, 92,919 decisions, with both: no failure. With defect 37's fix
  undone, the first decision of the first game is flagged, and a view
  leaking the other's first card through its hand size is flagged at the
  fourth. What the acting player is offered, with no question pending,
  is held to the same two changes: 553 games, 93,052 decisions, no
  failure; a menu naming the other's top card is flagged at the fourth.
  Then all three, mulligans, declarations and discards to hand size
  among the decisions held: 553 games, 94,325 decisions, no failure.
  Held at every decision they made a fuzz game nearly five times as
  slow (30 games in 66.8 s against 14.0 s), so the runner holds one
  decision in 32 to them, at 7% (15.0 s), inside the nightly shard's 90
  minutes; a leak that shows at all shows on most decisions of a game,
  and defect 37 undone is flagged at the 32nd. As it ships: 553 games,
  87,828 decisions, then 1,818 games, 301,531 decisions, no failure.

It found defects 37 and 38: a player's own library listed in the order it
lies, and the names of the cards of a question put to one player sent to
both.

### Cards proved with Kani

`mtg-engine/src/proofs/cards/` and `proofs/actions/`. A card proof runs
the card's own code, unchanged, on a board Kani leaves open (which
target, who owns and controls what, which zone each object is in, what
died this turn), with each engine action the card can ask for (destroy,
move, deal damage, draw, add counters, choose at random and some forty
others) replaced by a recorder. It asserts the exact list of actions the
card's text asks for, under the conditions it states, and by snapshot
that nothing else changed. What each action does is the other half,
proved once per action in `proofs::actions`: what the targeting
predicates answer, which targets a requirement offers, what a pending
effect does, which creature an Aura's or Equipment's effect reaches.
Every card with a hook of its own has a proof that calls it.

Kani proves these by bounded model checking through CBMC, and on this
engine's data CBMC's cost is set less by the code than by how the data
sits in memory. What was measured, and what the proofs do about each:

- **A list's growth.** CBMC cannot tell that a list has room before it
  encodes the answer that it has none, so every push encoded a
  reallocation of a size the data decides. Collecting a filtered list of
  three numbers took 59 million clauses with that and 0.7 million
  without. Every proof stands in for growing an allocation with a
  failure (`no_growing`): a proof that passes is one in which no list
  grew, and one whose lists must grow fails, naming it. Boards reserve
  their lists up front.
- **References kept on the heap.** CBMC keeps a heap allocation as bytes,
  and a reference read back from bytes is one whose offset it has lost:
  every read through it chose among every byte of every value it might
  point into. Under Kani the engine's map keeps each object in an
  allocation of its own (`vec_map`), so such a read chooses among
  objects at known offsets, and the stand-in for `objects_in_zone` hands
  its list out of static memory, which CBMC keeps as references.
  Resolving Make a Wish went from out of memory to 5.8 million clauses.
- **A heap buffer over 4096 bytes** is one CBMC no longer follows field
  by field (`--max-field-sensitivity-array-size`); eight objects in one
  buffer made a board-only proof take fifteen minutes.
- **Self-referring types.** Dropping, cloning or matching a filter or an
  effect read from the heap goes into every variant, recursively, to
  the unwinding bound: dropping a one-element list of a filter took a
  million steps. The proofs are run (`scripts/kani-bounded.py`, which
  `scripts/prove.sh kani` calls) with a recursion bound of three on every
  function over those types (the deepest filter in the pool nests two
  deep), with CBMC's recursion unwinding assertions left on, so a bound
  too small for the data fails the proof rather than passing it; that
  took the drop to eight thousand steps. The bound limits the search;
  the next item is why there was a search at all.
- **Enums written to the heap.** Kani encodes an enum as its tag beside a
  union of its variants, and builds a variant with its padding left
  nondeterministic and assigns it as one member of that union, over the
  union's bytes as they were. CBMC propagates a value only when it knows
  all of it, so neither is propagated: a variant copied to the heap
  through a parameter (`Box::new(x)`, `Vec::push(x)`) reaches memory as
  bytes CBMC cannot trace to the pointers inside it, and a pointer read
  back from there dereferences to an unconstrained object. The answer is
  unchanged, but every branch on what that object holds is explored, and
  a recursive function over a filter explores every variant at every
  level. Reproduced in eight lines of C: a variant written over a union
  with unknown padding loses its pointer; with zero padding it keeps it.
  `scripts/precise-enums.py`, which `kani-bounded.py` applies before CBMC
  reads a harness, zeroes that padding and makes each variant write a
  write of the whole union, as the bytes of the variant followed by zeros.
  Neither is observable by the program: safe Rust never reads padding or
  a variant other than the one its tag names, and the engine has no
  `unsafe`. Niche-encoded enums and Rust `union`s, whose other bytes a
  write keeps, are left alone. Cloning `And(vec![Not(Box(Ty))])` went
  from three million steps and out of memory to under a second; a
  recursive walk of a boxed filter from a five-minute timeout to three
  seconds. Spare from Evil's proof now finishes symbolic execution
  (1.7 million steps, 800 seconds), where it did not before, and runs
  out of memory in the solver; what is left of its cost is being
  measured.
- **Lists of a length the solver leaves open.** Cloning one allocates a
  buffer of that open size, which CBMC hands the solver as an array of
  that size; the action log is read in place and the recorders keep
  their lists at a fixed size. One that did not, the ids of the tokens a
  card asks for, allocated at the number asked for: Spider Spawning's
  proof finished symbolic execution and ran out of memory in
  propositional reduction, 4.65 million clauses, at four and a half
  gigabytes; with the list allocated at its largest first it passed at
  three and a half, 3.9 million clauses, in 349 seconds. And a list
  filtered and collected is allocated at the first entry kept, which the
  solver keeps or not: the lists a proof expects are filtered into a list
  allocated first (`kept`, and `picked` for a filter and a map). The
  destroy-all recorder's outcomes, allocated at the length of the ids it
  was handed and cloned for the record, were two more: with both of
  `MAX_LISTED` Paraselene's proof, which had run out of memory at four
  and a half gigabytes, passed in 242 seconds. The engine's own list of a player's
  creatures, which Selfless Cathar, Rally the Peasants, Vampiric Fury
  and Spare from Evil read, was a filtered `collect` too; allocated at
  its largest, Selfless Cathar's proof passes in 160 seconds at the cap
  it had run out of memory at.
- **An object a zone's listing may hold.** A card that scans a zone
  (Full Moon's Rise: each Werewolf creature its activator controls) is
  handed the zone's objects in a list whose every entry, with each
  object's zone and controller the solver's, may be any of the board's
  objects, and every read through one splits as many ways. Of the
  symbolic execution traced of Full Moon's Rise's ability, 45 per cent
  was `has_subtype` and a quarter `is_creature`, each read three ways,
  with every subtype compared byte by byte; it finished symbolic
  execution in 1.35 million steps and ran out of memory at three and a
  half gigabytes, the Werewolves and the activator constants. Vampiric
  Fury's, a constant Vampire or not on each object, ran out at four in
  symbolic execution. Making each object's zone and controller constant
  as well would be eight harnesses for each case there is now; these
  are run instead with more memory, alone.
- **Strings the solver cannot compare.** Nevermore offers every nonland
  card name in order, and sorts them: with the names on the heap, Kani
  could not tell how any two compared, and the sort of three did not
  finish symbolic execution in half an hour. The sort is now
  `helpers::sorted`, which a card proof stands in for with
  `names_in_order`, returning the names as they are and asserting they
  are in order, which is what a sort does to such a list; the proof's
  registry, kept in key order under Kani, hands them over so.
- **A card's whole data built to read one field of it.** `face_data`
  builds the active face's `CardData`, abilities and all, and a caller
  asking it one thing (is this a creature card, is it basic, what does
  it cost) dropped the rest again. A dropped `CardData` whose triggered
  abilities the solver cannot tell apart is dropped as each shape a
  target requirement may have, recursively: in the Civilized Scholar
  profile that was 70 per cent of symbolic execution, from one
  `is_creature_card`. The engine did the same in its hottest reads: the
  legend rule's `is_legendary` of every permanent at every check,
  `subtypes_of`, `colors_of`, `name_of`, a transformed creature's power
  and toughness, every permanent's triggers when a creature enters or
  dies, and a transform's test for a back face. The registry keeps each
  face's printed fields at registration, and those reads now go through
  `face_card_types`, `face_supertypes`, `face_subtypes`, `face_cost`,
  `face_name`, `prints_trigger` and the rest, each held to `face_data`
  for every card face up and face down by
  `the_kept_faces_say_what_face_data_says`.
- **A hash set's keys.** Back from the Brink offered one ability per
  creature card name and kept the names seen in a `HashSet`, whose
  SipHash keys are random, so the proof took them as any keys at all:
  symbolic execution in 23 seconds, and the solver unfinished at half an
  hour. A graveyard is a few dozen cards; the names seen are now a list,
  the abilities the same and in the same order. No other card hashes.
- **A clone of a value whose variant is open.** The proofs compared an
  activated ability with the one expected by building an `Ability` of
  the engine's, which cloned its target requirement, a type that holds
  itself; read out of a list built in one branch or the other, it is of
  a variant the solver leaves open, and the clone went down every shape a
  requirement can take. It was half of Moorland Haunt's symbolic
  execution, and where it was when it ran out of memory at four and a
  half gigabytes (and at ten). `Ability::describes` compares each field
  where it lies, the expected value on the left: each case of Moorland
  Haunt's now passes in two minutes.
- **Names placed by comparing them.** The proofs' card registry kept its
  names in key order, finding each one's place by comparing it with the
  others, which on the heap Kani cannot do: every name's place, and every
  id read back by position, was open. It keeps them in the order the
  cards were registered (`NameList`), asserting each name is new.
- **A case in its own harness, and a board in its own case.** The card
  proofs that read a list whose length the board decides (the abilities
  of a card by where it is and which face is up, a hand, a library, the
  creatures each player controls, the answer to a question) are each a
  harness for every value of what decides it: Caravan Vigil's search
  for each of sixteen libraries under either player, Divine Reckoning
  for each of 27 boards under either active player, Frightful Delusion,
  Nevermore, Ludevic's Test Subject, Bloodline Keeper, Civilized
  Scholar, Thraben Sentry, Cloistered Youth, Screeching Bat, Mentor of
  the Meek and Grimoire of the Dead the same way.
- **The zone order** itself is proved with Verus, for every board, not
  with Kani (above).

- **Loops whose count the solver leaves open**, such as collecting the
  creatures a player controls, were unwound to the harness's bound of
  seven however few objects a board held, each pass scanning the board
  again. `kani-bounded.py` unwinds every loop to four first and gives the
  harness's bound only to a loop whose unwinding assertion fails there,
  keeping the loops so raised for the harnesses after; the assertions
  stay on, so a harness passes only with every loop unwound as far as it
  goes.
- **Counters.** Adding a counter to a `BTreeMap`, and then to a `VecMap`
  whose two kinds of counter sat in an order the solver left open, ran
  CBMC out of memory: a write at an offset it leaves open makes it follow
  the buffer as a whole. A Kani build keeps an object's counters, and a
  player's mana pool, in one slot per kind (`kind_map`: six kinds of
  counter, six types of mana), held to an ordered map keyed by kind by
  `proofs::kind_map`, and every access is to a slot it knows.
- **Which side of `==`.** A derived `==` branches on its left operand's
  variant, and an action recorded at a place in the log the solver leaves
  open is of a variant it leaves open: with it on the left, every
  variant's fields were compared, two thirds of the mill proof's work.
  The proofs put the expected value, whose variant is known, on the left.
- **Lists of expected actions.** That is so only for an expected value
  built where it is compared. A list of them whose length the board
  decides (one action per creature the spell reaches), built first and
  then compared, is read past its end on paths the solver cannot rule
  out, and an action read from there is of a variant it leaves open:
  three quarters of the symbolic execution of milling an empty library
  was that comparison, and it ran out of memory. In a toy of six
  variants the same comparison ran out of memory where an expected value
  built in place took 25 seconds, and one built as either of two
  variants four times that. The proofs compare such lists run by run,
  each expected action built in place and of one variant
  (`asked_each`, `were`), or compare inside the branch where the
  list's length is known.
- **A board whose shape is left open.** The same holds of the engine's
  own lists: a list pushed to or not as the solver chooses has its next
  entry written at an index it leaves open, and an entry read back from
  there (to drop it, compare it, walk it) is of a variant it leaves open.
  Countering a spell under another spell or not went, as it dropped the
  countered entry, into every variant of a stack entry and every target
  requirement an ability's might hold, three deep. A proof whose board
  has a few shapes calls its body once per shape with the shape a
  constant (`countering(on, under)`), and the same for a player the
  engine indexes its players by (`move_object` reaches the owner's zones
  that way). A board of cards that are there or not makes them all and
  puts the absent ones where the card does not look (`cards_in_zones`),
  so that every object's id is known: Desperate Ravings went from a
  timeout to 201 seconds. The same three things, a list of a length the
  solver leaves open, an object it leaves open, a player it indexes the
  players by, were behind most of the proofs that ran out of memory
  since: a target that may have left the game (an id one past the
  board's: destroying), the object each effect is about (pruning), a
  planeswalker made or not (it moves the ids after it), how many tokens
  and whether the opponent is in the game (tokens entering attacking), a
  library of open length and cards (Mindshrieker, Cellar Door,
  Heretic's Punishment, which then read each card's printed data through
  an index into the registry), a hand of open size (Grimoire of the
  Dead). Each is a constant in each call or each harness, every case
  still proved. An effect copied whole into the heap with a number in it
  the solver chose is bytes CBMC does not take apart (it propagates no
  value holding a symbol), so the printed effects are proved as the
  pool's cards print them; and where an amount the solver chose may be 0
  and is then never queued, the queue's length is open whatever else is
  constant, so the queue is proved with what it hands on recorded, and
  what it hands to (`damage::perform`) on its own.
- **The cases of one harness.** Making a choice a constant in each call
  runs each case's symbolic execution in full, and the cases of one
  harness add up. One case of Traveler's Amulet's board peaked at 2.1
  gigabytes and passed in four minutes; two in one harness ran out of
  memory at three, whether run in turn under a guard each, as an `if` and
  its `else`, or as a `match`; one under a guard alone passed as the
  bare one did. So a card proof's cases are harnesses of their own
  (`case_by_case!`, which names one harness per case), and a choice that
  decides no list's length and no index into the players stays the
  solver's, in one call: the targets of Avacynian Priest's and Evil
  Twin's resolutions, which ran out of memory as five cases, pass as one
  (158 and 213 seconds).
- **What is never written.** `precise-enums.py` also zeroes every heap
  allocation, as `calloc` does, and the bytes outside a niche-encoded
  enum's variant (its tag kept in another variant's bytes) where Kani
  writes the variant's fields, or none, and then the tag. Safe Rust reads
  no memory it has not written, so neither is observable; what is read on
  a path the solver cannot rule out is then one known variant rather than
  every one.
- **A rewrite's own cost.** Writing a variant as the bytes of its whole
  union, the rest zero, is what lets CBMC follow an effect into a `Box`;
  but a narrow variant holding a pointer, laid over a wide union, is more
  than it gets through. Asking a yes/no question,
  `awaiting_action := Some(ResolutionChoice { YesNo })` (a 256-bit
  variant in a 1152-bit union), ran out of memory at three gigabytes,
  and passes in sixteen seconds as Kani wrote it; every card proof whose
  card asks one (Cloistered Youth, Thraben Sentry, Screeching Bat, Delver
  of Secrets) ran out of memory in symbolic execution, Cloistered Youth's
  after nine minutes at four gigabytes. Such a write is now left as
  written over a union wider than 1024 bits, and Cloistered Youth's
  passes in 86 seconds; the effect proofs the rewrite was for hold in the
  time they did.
- **A path that cannot be taken.** Where CBMC cannot tell that a path is
  impossible (moving a card onto the battlefield, on a board where no
  replacement decides a copy, still went into becoming a copy, two
  thirds of the proof), the function there stands in as an assertion that
  it is not called (`no_copy`): the proof then holds only if no path
  reaches it, and the solver, not symbolic execution, settles that none
  does.
- **Which member of a union.** CBMC splits a union into its members and
  keeps a value written as bytes as its widest member, but reads through a
  pointer, or by bytes, through the *first* member with a field where
  those bytes are, and by name through the member named. A read through a
  member other than the one kept is simplified only by turning the whole
  value into bits, which CBMC cannot do when the value holds a pointer
  (any `String`'s) or bytes never written (a fieldless variant's). So the
  tag of a niche-encoded enum (`PendingEffect`, `TargetFilter`,
  `GameEvent`: the tag kept in a niche of one variant's bytes) was not
  known in symbolic execution even three lines after the value was built,
  and every `match` on it followed every arm: applying a -1/-2 went into a
  token's attack and, recursively, the choice of the next one, out of
  memory. And a tagged enum's variant inside another's
  (`ContinuousEffect::PreventAttack { scope: EffectScope::Attached }`)
  was cloned as every scope, each filter's every variant included, out of
  memory. In a Kani build the engine's niche-encoded enums are tagged
  (`#[cfg_attr(kani, repr(u8))]`, a change of layout only), and
  `precise-enums.py` makes a fieldless variant's unwritten bytes zero, so
  that CBMC knows, and propagates, all of a value holding one; and, for
  the niche-encoded enums left (the standard library's), puts each
  union's widest variant first (a union's members all start at its first
  byte, so their order is not the program's) and writes the tag as part
  of the variant's value. (Writing a tagged variant as the union of its
  own member instead, which a read by that name gives back exactly, made
  a value copied whole, into a `Box`, unknown: CBMC splits a union so
  built into its members and keeps no whole.) The debuff harness then
  finished in 75 seconds, which had run out of memory at six gigabytes,
  and the clone of that effect, boxed or not, in some sixty thousand
  steps, which had run out of memory at three.
- **A pointer one past the end.** CBMC simplifies `&x == NULL` to false,
  but not `&x + 80 == NULL`, the test of whether an `Option` whose niche is
  a pointer is empty when that pointer is one past the end of a list. A
  `Chain` of two slice iterators keeps its first half as such an
  `Option`, and once the first half had run out, symbolic execution
  took it as possibly absent and read the next effect through null, from
  an object CBMC leaves open. Walking the effects that reach a second
  creature then went into every effect, scope and filter, out of memory
  (the first creature's walk stopped at the Aura's effect, which reached
  it). `precise-enums.py` also compares such a pointer's offset with zero,
  which CBMC does simplify, to 80. Null is the null object at offset zero,
  so the test is the same test; "can't attack" and its three siblings
  then ran in 37 seconds where they had run out of memory at five
  gigabytes, and the claim that the second creature can't attack fails
  as it should.
- **An `Option` passed on by `?`.** The `?` of an `Option` returns
  `ControlFlow::Break(None)`, which Kani builds as a declaration, a write
  of its empty `Break` member and its niche tag; `precise-enums.py` made a
  `None` whole only when the tag followed the declaration, so the list in
  `Continue` beside it was of unknown length, and the tap plan the
  planner passed on through `?` was too wherever its paths met. A payment
  from the pool, which taps nothing on every path, then walked a plan of
  any length, every card's mana abilities with it, out of memory at 248
  thousand steps. A write of a member of no bits may now come between.
  The plan's pointer still merges two values (the dangling one, or the
  `None`'s null), so the iterator's `ptr == end` stays open where the
  length is known: the payment walks its plan by index.
- **A walk by value of a wide enum.** `global_effects` cloned every
  object's effects and walked the copy by value, an `Option` of a whole
  `ContinuousEffect` per step, which CBMC holds field by field, every
  variant's every field; a payment asks after Stony Silence at each
  source. It reads them in place, as `walk_effects` does.
- **A zone listed to be counted.** A card counting or testing objects in
  a zone built the zone's list first, whose entries a proof reads as
  pointers that could be any object's, as many as who holds each
  decides; comparing subtypes through them ran Bloodline Keeper's five
  Vampires and the check lands' Mountain or Plains out of memory at
  fifty seconds. `count_in_zone`, `any_in_zone` and `ids_in_zone` scan
  the objects in id order with no list built (a count or an any has no
  order; outside a graveyard the zone's order is id order), and
  `objects_in_zone` itself lists every zone but a graveyard by that scan:
  with no pile, the kernel's order is the members in id order. Bloodline
  Keeper's ability proofs then passed in 101 to 174 seconds, the check
  lands' in about a minute, Endless Ranks of the Dead's in 384 seconds,
  Gavony Township's in 185.
- **Text built for a log line.** Night Terrors joined the names of every
  card in the hand into a string before the line was formatted, which a
  proof follows name by name; a `Display` of the ids writes them only
  when the line is (formatting stands in as nothing in the proofs).
- **A zone listed to be filtered.** Offering targets built the
  battlefield's list of references and filtered it, each question a
  filter asks being asked through an entry that could be any object.
  `each_in_zone` and `each_object` walk the map in id order with no list
  built, as `count_in_zone` does.
- **A string cloned to compare.** Protection from a subtype built the
  source's subtype list, a clone of every string, and searched it; it
  asks `has_subtype` of each protected subtype instead.
- **A copy read through a pointer of more than one target.** Where
  symbolic execution cannot tell which object a pointer points into (a
  list held in an engine prompt, read back through the prompt's union),
  CBMC's `memcpy` copies the first byte and leaves the rest open ("What
  is trusted" says why that is sound). So a clone of a prompt, or of a
  string or a list of ids in one, is a value of any contents, and the
  proof explores every one: the choice handler cloned the whole prompt
  to put back on a refused answer, the damage queue cloned each event
  with its list of effects already applied, and Nevermore cloned the
  name chosen. Each now reads what it needs where it lies, puts back the
  prompt it took, or moves the string out of the spent prompt.
- **A branch an assumption rules out.** Symbolic execution drops a
  branch only where its condition comes out a constant; `kani::assume`
  narrows what the solver may pick, not what is walked. The proof of
  prevention and doubling assumed the damage 1 to 3, and `queue_damage`'s
  branch for 0 damage, which queues nothing, was walked all the same;
  where it met the branch that queued the event, the queue's length was
  0 or 1 rather than 1, and the settling loop was walked for every
  length, the orders of every list of effects compared, out of memory at
  3.5 GB. With the amount a constant in each harness, the four of those
  harnesses tried passed in 87 to 124 seconds.
- **A list as long as the answers.** The same join, for a list: one
  pushed to only for some answers is, where the branches meet, of every
  length the answers allow, and a loop over it is walked to the loop's
  bound. Destroying all listed the deaths that way and walked the list to
  announce and move them; it keeps one entry per permanent, none for one
  that does not die, and both forms passed in 264 and 286 seconds where
  they ran out of memory.
- **Questions filed in a list.** `decided` kept the answers given so far
  in a list and looked each question up in it. Which questions came
  first depends on the answers, so where branches met the list differed
  by path, and every later question was compared with every entry of
  every list: 72% of the targeting proofs' symbolic execution. One place
  per question and object, found by index, holds the same answers; the
  filtered-creature proof went from 1.3 million steps to 480 thousand.
- **A filter walked to its end.** `Iterator::filter` hands out its items
  by an inner `find`, so after the first item the walk stands at one of
  as many places as items passed, and every later item, and each
  question asked of it, is read at a place of every value. Offering
  targets collected chains of filters over the battlefield and the
  players; a loop over `each_object` with the tests in its body asks the
  same questions in the same order and reads each object at its own
  place. The filtered-creature proof's symbolic execution fell from 121
  seconds to 11, and it and the filtered-permanent proof passed in 44 and
  41 seconds. With players offered as well (any target, player or
  planeswalker) the list is pushed at a place of every value up to four,
  and those two still run out of memory in the solver at 3.5 GB.
- **An effect held and cloned.** The scope proofs' card held its
  continuous effect and cloned it into its card data. The clone's kind
  came back exact and its scope did not: protection from Zombies,
  printed on itself, was matched against every filter tree the scope
  might have been, and it and "Spirit creatures you control" ran out of
  memory. A card that builds the effect where it prints it (the pool's
  cards all do) gives a constant; those four proofs passed in 34 to 39
  seconds.
- **A prompt dropped.** A card proof that had checked a prompt and went
  on as if it were answered cleared it by assigning `None`, which drops
  the old prompt; one symbolic execution cannot tell from every other
  kind of question is dropped as each of them, a mana-funding plan's
  B-tree map among them. `clear_prompt` takes it out and forgets it
  (freeing is nothing in a proof). Cloistered Youth's, Thraben Sentry's
  and Civilized Scholar's proofs then passed in 109 to 228 seconds where
  they ran out of memory; Graveyard Shovel's and Creeping Renaissance's
  finish symbolic execution and still run out of memory in the solver.
- **A B-tree followed.** A card's state was a `BTreeMap` in every build,
  and a proof follows a B-tree's rebalancing: 57% of Divine Reckoning's
  symbolic execution was in removing its kept ids. It is a `VecMap` in
  the proofs (`CardState`), as the objects and counters already were.
- **An entry moved as bytes.** `VecMap` shifted its entries with
  `slice::swap`, which copies two values as bytes, eight at a time, and a
  proof reading a key's string back from bytes so copied no longer knows
  which allocation it points into: after an insertion out of key order,
  every comparison of a key was of any string and every value any value.
  Each entry is now read out and written back whole. Divine Reckoning's
  stored list reads back as written, and its proofs where a player chooses
  finish symbolic execution in 51 seconds (they still run out of memory in
  the solver at 3.5 GB); its proof over a board of no creatures, listing
  the battlefield by scanning it rather than through a list of references,
  passed in 261 seconds.
- **A sort walked.** The standard sort (driftsort, its runs and its
  networks of copies) is walked for every length the list might be.
  The payment proofs were still in symbolic execution after an hour in
  `greedy_autotap`'s sort of a handful of pips and sources; a stable
  insertion sort, each key computed once, gives the same order, and they
  finish symbolic execution in about 70 seconds (and run out of memory in
  the solver at 3.5 GB). A proof's own sort of ids already in order
  (Kessig Cagebreakers', 15 minutes and still walking) is an assertion
  of the order instead.
- **An object looked up by an id read back from a prompt.** A choice
  that is a narrow variant of a union wider than 1024 bits is written as
  that member (`precise-enums.py` says why), so the prompt holding it is
  not propagated, and every field read back out of it is one symbolic
  execution cannot pin down: Nevermore's object id among them. A lookup
  by such an id hands back a reference that is to the object on one path
  and null on the other, a read through it is the object's field or an
  unconstrained one's, and assigning the chosen name over the object's
  old effects dropped those as any value, every variant of every effect
  and filter. The handler finds the object by scanning the objects (a
  reference the scan hands out is to one that exists), and Nevermore's
  six proofs pass in 148 to 171 seconds, where they ran out of memory in
  symbolic execution.
- **A prompt's lists read back for a second answer.** Liliana of the
  Veil's −6 asks two questions, the second with the piles the first
  answer made, and its handler loops over those piles as read back out
  of the prompt: lists of a length symbolic execution cannot pin down,
  carrying every expression of the first answer. Both answers in one
  harness ran out of memory in symbolic execution, a constant board and
  division notwithstanding. Each answer has harnesses of its own, the
  second from the question as the first's proof finds it (its handler
  reads of it only what that proof checks), and all 39 pass in 146 to
  238 seconds.
- **Lists kept in a card's state.** A card that waits on several answers
  keeps lists between them under string keys, built byte by byte, in a
  map a proof follows comparison by comparison: three quarters of
  Liliana's +1 proof, every case a constant, and out of memory at seven
  gigabytes. The card proofs keep the lists in a table of their own
  (`record_store_ids`), and `proofs::actions::card_state` proves the
  helpers keep them as the table does. The +1's cases then pass in
  about four minutes at 3.5 GB, and so does a case of Divine
  Reckoning's that ran out of memory in the solver.
- **An array of no size.** Kani does not follow a zero-sized array: each
  proof of the kept lists that built an empty one with
  `core::array::from_fn` ran out of memory, where those of one to three
  ids passed in under a minute. Built as slices of a three-id buffer,
  the empty ones pass in 9 to 39 seconds.
- **A list rotated.** Divine Reckoning put its players in turn order by
  rotating the list of them to start at the active player, and a
  rotation moves its entries as bytes through a buffer of its own: with
  the second player active (the first, the rotation is a no-op), every
  player read back was either, and each proof where the first player was
  asked ran out of memory in symbolic execution, where the same boards
  with the first player active passed. Read in that order instead, the
  players are each one known, and those proofs pass in about 270
  seconds.
- **An action recorded or not.** Moldgraf Monstrosity exiles itself only
  if it is still in its graveyard, and its proof left that to the
  solver: whether the exile was recorded left every action after it at
  a place the solver left open, each written over whatever might have
  been there and each read back as any kind of action, a token's
  subtypes and all. With the zone a constant in each harness, both
  finish symbolic execution in about 65 seconds (and still run out of
  memory in the solver at 3.5 GB).
- **A filter walked again, in a card.** The cards that return a card at
  random from a graveyard (Woodland Sleuth, Ghoulraiser, Charmbreaker
  Devils, Moldgraf Monstrosity) found their candidates as a filter
  collected, which a proof walks again for each one it keeps, looking
  each card up by id each time. As loops, their proofs' symbolic
  execution halves; Charmbreaker Devils' passes at 7 GB, and Woodland
  Sleuth's too once its loop is held to the five passes its board needs
  (`// loop bound:` now takes a trait method's name, spaces and all)
  rather than the harness's seven. Copying each chosen card's name for
  the log line, which looked like the cost, was not: borrowed instead,
  nothing moved.
- **An effect compared whole, again.** Garruk, the Veil-Cursed's −3 and
  the pruning of departed objects' effects compared each effect they
  checked with `==`, over effects read back from a list the solver had
  pushed to, or the pruning had moved, to symbolic execution of any
  kind: half and seven eighths of those proofs. Matched by kind instead,
  as Vampiric Fury's was, Garruk's passes in 372 seconds at 3.5 GB
  (from out of memory at 7) and the prevention's pruning in 70 (from out
  of memory in symbolic execution at 7).
- **A write at a place the solver leaves open, again.** CBMC follows a
  heap buffer of up to 4096 bytes byte by byte (see above), and a write
  to it at an index it leaves open is a write of every byte, each a
  choice among all of them: the cost of a push goes with the square of
  the buffer's size, not the entry's. Vampiric Fury pushes two 64-byte
  effects per Vampire onto a list the board gave room for eight, and
  those pushes were two thirds of its formula (`cbmc --program-only`,
  sized by function). With room for the four it can push, the formula
  halved (37 to 19 million characters). A proof that pushes in a loop
  gives the list the room it needs and no more.
- **An array mapped.** `[T; N]::map` goes through `try_from_fn` and an
  array `Drain`, which CBMC follows, and what comes out is a value it
  leaves open: Vampiric Fury's subtype lists, picked by
  `vampires.map(...)`, made every check of a subtype with them a
  question (731 thousand steps of symbolic execution; 558 with each
  list picked by an `if`). The proofs build such arrays element by
  element. `Snapshot::of` still maps two arrays, and every card proof
  goes through it; changing it would mean proving all of them again.
- **Two assertions, one harness.** Vampiric Fury's effects, and that it
  did nothing else, each passed at 6 GB in a harness of its own (about
  seven million clauses), where the two in one harness ran out of memory
  at 12. A proof whose parts each read the whole board is split into a
  harness per part; together they say what the one said.
- **A board's length, and its owners.** Mulch's library of up to five
  cards, its length the solver's, was 1.45 million steps and out of
  memory at 12 GB; with a harness per length, none to three cards pass at
  3.5 to 7 GB, and four and five reach the solver at 7 with 21.5 and 21.8
  million clauses. Past in Flames' three cards, each in either player's
  graveyard, gave a graveyard listing the solver left open and copies of
  each card's cost of lengths it left open; with the owners, and whether
  the instant already had flashback, constants of each of sixteen
  harnesses, each passes at 6 GB in about three minutes (1.6 million
  clauses). Counting a cost's pips over a length the solver chose ran out
  of memory at 3.5 GB; a harness per length takes one to four seconds.
- **A payment planned twice.** `pay_cost_with_sources` plans before it
  taps, so a proof that checked the plan and then paid ran the planner
  twice. The Forest's plan alone, over a pool holding a constant amount
  of the cost's colour and the Forest untapped or not as a constant,
  passes at 7 GB (1.7 million clauses); over a pool and a Forest the
  solver chose, it runs out of memory at 7 in symbolic execution. Paying
  by tapping the Forest is the weight: activating its ability (more than
  half of what symbolic execution traced) built the Forest's list of
  abilities as a filter collected and dropped it, and joined a log line's
  description of the pool that the proof never reads. Written without
  either, paying took 1.82 million steps rather than 2.50, and still ran
  out of memory at 7 GB.
- **A sort of what is in order, and room of an open length.**
  `creatures_controlled_by`, which three cards ask whom to sacrifice
  from (Stitcher's Apprentice, Tribute to Hunger, Liliana of the Veil's
  −2), gave its list the battlefield listing's length as its room and
  then sorted it by id. The listing is in id order already
  (`objects_in_zone`, its order proved with Verus), so the sort changed
  nothing, but a proof walked its swaps over places it left open, and
  the room was a buffer of a length it left open. Without either,
  Liliana's −2 over a player went from out of memory at 12 GB to 7.3
  million clauses at 7, and Stitcher's Apprentice and Tribute to Hunger
  pass at 7.
- **A board of shapes, and a list of types the solver leaves open.**
  Garruk, the Veil-Cursed's −1 sacrifices one of its activator's
  creatures, and over two objects of any characteristics their list of
  creatures was one the solver left open (out of memory at 12 GB, 50
  million clauses). On a board whose objects are each of a shape to that
  player (a creature of theirs, an enchantment of theirs, the other
  player's permanent, a card in a graveyard), the rest of their
  characteristics any, 23 of the 24 cases pass at 3.5 GB in 160 to 384
  seconds (576fd1f gives 180 to 433, an earlier run's). A noncreature
  had first been given types any of an enchantment's or none, and with
  its list of types of a length the solver chose, whether it held
  Creature was a question even where it could not: every case with one
  ran out of memory, every other passed.
- **An engine action stood in for, inside an engine action's proof.**
  Paying by tapping a Forest activates the Forest's ability, and that
  activation was more than half of what the payment's proof followed.
  The activation is proved once on its own (tapped, then {G} added,
  nothing else), and a stand-in that does exactly that takes its place
  in the payment's proofs, as the card proofs' recorders stand in for
  the engine actions proved beside them: the payment went from out of
  memory at 7 GB in symbolic execution (2.5 million steps) to 984
  thousand steps and 13.8 million clauses.
- **A generic pip over a pool the solver chose.** With the activation
  stood in for, paying {1}, and {1}{G} with green floating, by an
  untapped Forest still ran out of memory at 12 GB (41 million clauses),
  {1} with its green held to a constant as well: paying a generic pip
  orders what the pool has beyond the colours' needs to choose what to
  spend, and over amounts the solver left open the order was the
  formula, even where the pool paid alone. Over each pool constant
  (white, green and colourless each from none to two, for {1}{G} with
  one or two green, for each player: 54 harnesses for {1} and 36 for
  {1}{G}, together the pools the one harness was over), each is 1.3 to
  2.3 million clauses and passes, at 3.5 GB or, the four that tap the
  Forest, at 7, in 154 to 368 seconds.
- **A list of clones, read and dropped.** What a spell costs read the
  cost changes its caster's permanents print through
  `effects_controlled_by`, which collected a clone of every effect; the
  list, dropped once read, was dropped as every shape of effect and filter
  there is, and Heartless Summoning's reduction ran out of memory at 3.5
  GB in symbolic execution. Visited where they lie
  (`visit_effects_controlled_by`), and the spell's types read off its
  printed face rather than its whole data built, each cost passes in
  about thirty seconds. `global_effects` collected clones the same way,
  and Stony Silence's and Nevermore's proofs, which read it, ran out of
  memory at 7 GB; visited in place (`visit_global_effects`, 06a47c7,
  Nevermore's names borrowed from the effect), they pass in 70 and 30
  seconds. Every proof of paying follows it, and is run again (below).
- **A sort of a list of open length.** Which effects apply to damage
  (Ghostly Possession's, Unbreathing Horde's, Inquisitor's Flail's,
  Moonmist's) collected each list and then sorted and deduplicated it: a
  proof that leaves the list's length open follows the sort through every
  length's sorting network, four fifths of what Inquisitor's Flail's proof
  cost. Built in order as it is found (`insert_once_in_order`), the lists
  are the same and the sort is gone.
- **A filter cloned, then matched.** "Can't be blocked except by" was
  kept as a clone of its filter for each attacker, and a filter cloned off
  the heap is one a proof cannot tell from any other: matching a blocker
  against it matched every filter there is. Borrowed where it is printed
  (`walk_effects` names the lifetime its visitor's effects live for), and
  the card that prints it in a proof building its effect where it prints
  it rather than holding one and cloning it (`effects::Builds`, as
  `scopes::Prints`). `Protections` cloned its filters the same way and
  borrows them now too (70e09e5; `can_be_targeted_by` asks it, so every
  card proof that lists targets is run again, below). Neither was enough for
  Stromkirk Noble's and Blazing Torch's, nor for Spare from Evil's
  protection or Moonmist's prevention, which still ran out of memory in
  symbolic execution, at 12 GB as at 7: profiled, matching the blocker
  against the only-by filter was three quarters of the work, the filter
  behind a `Box` in an effect kept on the heap matched as every filter
  there is (keywords, card types, colours), and `precise-enums.py` does
  not reach a filter there (a tag of its own for the filter, which would
  let it, made other proofs worse when tried, below). Built where it is
  matched, though, a filter is matched as itself: so what
  `matches_filter` makes of each filter a card prints is proved on its
  own (`proofs::actions::filters`, five filters, 21 to 25 seconds each,
  each question it asks any answer), and the proofs of the effects stand
  in for the filter with any answer, checked to be asked only of the
  effect's own filter, the same object by pointer
  (`filters::stand_in`, 7f59844). Blazing Torch's and Orchard Spirit's
  restrictions then pass in 144 and 146 seconds, Spare from Evil's
  protection in 46 and Moonmist's prevention in 75.
- **An expected list of open length.** The cleanup step's proof first
  built the list of control changes it expected by pushing one as each
  creature returned: a list whose length and contents the solver left
  open, and each comparison with what was recorded was against every kind
  of action there is (70% of the proof). Each case's list written out,
  of a known length, is what `asked_for` wants.
- **A card's whole data, built to read one part.** The trigger
  collector asked whether a card prints a trigger, and what it says, by
  building the card's `CardData` (`card_has_trigger`,
  `trigger_description`), and the stack asked a card for its target
  requirement. Each dropped value holds target requirements, and a
  dropped `TargetRequirement` read back so is dropped as every shape one
  may have, recursively: nine tenths of what the trigger collector's
  proofs and the stack's cost, out of memory at 7 GB. A tag of its own
  for `TargetRequirement` (`repr(u8)` under Kani) did not change that, and
  tags for the filters and effects as well (`CreatureFilter`,
  `EffectScope`, `ContinuousEffect`) made it worse: a scope proof of 41
  seconds ran out of memory at 7 GB with them. What did: a value built to
  be read is left undropped in the proofs (`cards::transient`, a24580f;
  nothing in a card's data has a destructor that does anything but free,
  and freeing stands in as nothing), and the registry's printed faces,
  which already list each face's kinds of trigger, are asked first
  (`prints_trigger`, 66df5a7, e4c0600), so a card printing no trigger of
  the kind builds nothing. The same answers, which the engine's tests
  hold.
- **An event of a kind the solver chose.** A proof of combat damage
  over combat damage and noncombat damage built the event as one or the
  other and handed it to the collector in a list on the heap: read back,
  it was an event of every kind, and the collectors for blocks, deaths
  and attacks ran too, three quarters of the work, out of memory at 7 GB.
  Handed a batch on the stack, one constant event down each branch, each
  passes in 59 to 101 seconds. The collector's own batch had the same
  shape: `collect_triggers` cloned the events to read them beside the
  state, and now holds them apart while the collectors run (0b53fcb).
- **Which list a trigger is filed in.** With the controller of an
  ability and the active player both left to the solver, which of the
  collector's two lists the ability goes in is a real question, and with
  two places it might be written at, the ability was written into each
  place of each list it might be at: three quarters of a formula out of
  memory at 7 GB. With the controller a constant, the delayed exile's
  resolving passes in 51 seconds; with the token's zone a constant too
  (left open, the step's own triggers were looked for on a list of
  permanents of a length symbolic execution could not see, and it built
  the prompt to order a queue of one), the end of combat's passes in 84.
- **A list inside a narrow variant.** An event's declared attackers are
  a list in `GameEvent::AttackersDeclared`, a variant narrower than the
  enum's widest, which `precise-enums.py` leaves as written: read back,
  the list's length is one symbolic execution cannot see, and the
  collector's loop over it ran to the starting bound, four passes, three
  over attackers it could not rule out, the attached Equipment's ability
  raised in each. Bounded to two passes (`// loop bound:
  triggers::collect::combat::attackers_declared#2 2`, a loop number now
  being something a bound may name, 133a83d), the unwinding assertion
  proving no third is taken, the four Equipment proofs pass in 174 to
  187 seconds.
- **A stack entry cloned to be read, and dropped.** Resolving the top of
  the stack cloned the entry to match on it, an ability's carrying its
  target requirement: cloned off the heap, every shape one may have, nine
  tenths of an ability's resolution. It is matched where it lies now
  (e253dfe), and the popped entry, kept as the resolving ability or done
  with, is left undropped in a proof (`cards::discard`).
- **Names listed for a log line, again.** A resolution built the names
  of the targets gone illegal, for the log alone (a join of them for the
  line first, then the list itself): pushed to along paths symbolic
  execution could not tell apart, each name written into a buffer it
  could not tell either rewrote every byte of each, and the solver ran
  out of memory reducing the formula. They are named when the line is
  written now (`NewlyIllegal`, f7ed1bc), which in a proof is never.
  With that, and the harness's own lists of targets held as arrays of
  ids (each list of targets cloned, walked and compared was pointers
  turned into numbers to check their alignment, which the solver relates
  to every object they might point into: 224 such in a spell's proof, 65
  in the delayed exile's), an ability's resolution passes in 49 to 80
  seconds. A spell's ran out of memory in the solver, at 12 GB as at 7:
  its targets are read back off its object, a list whose length symbolic
  execution does not see, and the re-check walked that list every way.
  Read and compared alone they prove in 42 seconds, and handed an empty
  list in their place the re-check's case of no target passed in 50. So
  the re-check is its own function now, `stack::resolve_spell_as`, handed
  what `resolve_spell` reads (3901288): its proofs run on it, and three
  more show that a spell on top of the stack leaves it and hands
  `resolve_spell_as` the spell, its card, its targets in order and its
  controller, each half passing in 47 to 76 seconds.
- **A union read back.** Flashback granted until end of turn ran out
  of memory at 7 GB, even with the granted effect on another card and
  nothing to offer: 24 million clauses at 3.5 GB, nine tenths of the
  formula one buffer, the turn's list of effects, rewritten at each
  symbol of each cost copied. An effect read back off that list is a
  union to symbolic execution, held as one variant (Moonmist's, a
  filter and a name, both on the heap), and which card a grant was on
  came back as bytes of that variant it could not reduce: so the costs
  offered were a list of a length it did not see, each copy of one a
  write it could not place, into every buffer it might be. Which effects
  are on a card is its own function now, `granted_flashback_costs`
  (proved by where each cost it hands back is, never what it holds), and
  the costs offered are proved with it standing in as that proof shows
  it answers, each cost read from the harness's own list: 0.3 to 0.65
  million clauses, 19 to 64 seconds. Cutting the costs' copies (four on
  the way to an offer) as well changed nothing that mattered and was
  left out.
- **A trigger read back off the stack.** A triggered ability's proof
  ran out of memory in symbolic execution, three ways in turn. Filed
  by the collector under a controller left to the solver, which of its
  two lists the ability went to was not settled (as `delayed` found):
  it is filed under a player of its own now and given its controller
  after. Read back off the stack, its list of targets was one whose
  length symbolic execution does not see, so the target re-check ran
  with no target, and looked up the requirement the card prints,
  cloned off card data rebuilt on the heap as every shape a requirement
  may take: the lookup stands in as none, which only the re-check's own
  stand-in question reads, and is proved apart. And the ability's name
  for the "resolved" line was built before the line, for no reader,
  substituting the event's objects into its description by string
  search: it is worked out when the line is written now
  (`PendingTrigger::logged`, 1050b6c). Each kind of event then proves
  in under a minute, but a creature's dying, whose event's lists are
  copied as the ability is kept resolving.
- **A copy of the whole game.** Casting a spell and activating an
  ability each rehearsed the tap plan on a clone of the game state
  before paying, a copy no symbolic execution follows. With nothing to
  tap the rehearsal is the pool as it is, and both read it so now
  (783ac6d, 37e4c42). What was left was the mana the rest of the hand
  wants kept back as the cost is paid (`demand_reserve`), which reads
  every card's cost on the board: three quarters of the cast's symbolic
  execution, and nothing a cost of no mana pays depends on it.
- **CBMC's memory checks.** `kani-bounded.py` leaves out CBMC's pointer,
  bounds and division checks (`--memory-checks` keeps them): that safe
  Rust reads only memory it may is the compiler's guarantee, and Rust's
  own checks are assertions in the program and stay. Rolling Temblor's
  symbolic execution took half the time without them, with 3876 checks
  left to solve rather than 9670. Freeing stands in as nothing in the
  action proofs as in the card proofs (`no_freeing`).

The engine actions these card proofs stand in for, proved so far (each
for both players and boards as its doc comment states):

| Action | Result |
|---|---|
| `change_life` and its four other forms (CR 119.3, 119.10) | proved, 41 s |
| `tap`, `tap_for` (701.21a, 110.5) | proved, 59 s |
| `untap`, `untap_for` (701.21b, 110.5) | proved, 60 s |
| `discard_card`, `discard_card_for` (701.8a) | proved, 91 s |
| `discard_cards`, N from a hand of 0 to 2 | proved, 70 to 159 s |
| `attach` (701.3a, 701.3b, 702.6c) | proved, 82 s |
| `add_counters`, `add_counters_quiet` (122.1) | proved, 86 s |
| `remove_counters` | proved, 74 s |
| `add_regeneration_shield` (701.15a) | proved, 51 s |
| `set_base_pt` (613.4b) | proved, 22 s |
| `gain_control_while_source_controlled` (613.1b) | proved, 97 s |
| `player_loses`, `end_game` (104.3) | proved, 37 s and 52 s |
| `draw_cards` (121.1, 121.2, 704.5b, 614.1), a library of 0 to 3 | proved, 120 to 231 s |
| `mill_cards`, `mill_one` (701.13a), a library of 0 to 2 | proved, 54 to 190 s |
| `put_into_library`, top or bottom, from the library itself or exile, a library of 0 to 3 | proved, 79 to 157 s |
| `shuffle_library` (701.20a), `choose_at_random` of 0 to 4, `flip_coin`, `random_below` | proved, 6 to 67 s |
| `counter_spell`, `counter_spell_exiling` (701.5a, 702.34a) | proved, 319 s |
| `apply_pending_effect`: damage, destroy, sacrifice, return to hand, -N/-N, can't block, a card's own effect, a search's answer | proved, 64 to 173 s each |
| the targeting predicates (`is_creature`, colours, card types, keywords, filters, `can_be_targeted_by`) | proved, 13 to 103 s each |
| an Aura's or Equipment's printed effect reaching the creature it is attached to (613.1, 611.3a): each P/T change, keyword and Human condition the pool prints, can't attack, can't block, attacks each combat, doesn't untap | proved, 36 to 76 s each |
| `try_destroy`, `try_destroy_by`, `try_destroy_no_regen`, `sacrifice_by` (701.7a, 702.12b, 701.15a, 701.17a), and `try_destroy_all`, `try_destroy_all_by` (each destroyed as it would be alone), each its own harness | proved, 146 to 285 s each |
| `present_target_choice`, `present_optional_target_choice` (a choice of target applied at once or asked) | proved, 121 s |
| `fight` (701.12a) | proved, 113 s |
| damage on a board printing no effect: only protection applies (615) | proved, 62 s |
| tokens entering attacking (506.3, 508.4), a token on a board without a planeswalker | proved, 161 s, at seven gigabytes |
| `prune_effects_on_departed_objects` (400.7): each kind of effect about either object or one gone, and two or three in order | proved, 52 to 198 s each, but the prevention's (past four gigabytes) |
| `damage::perform` (120.3, 702.2b, 702.15b) on either player | proved, 212 and 219 s |
| damage nothing prevents or replaces, through the replacement layer (616.1), to either player | proved, 89 and 91 s |
| `search_library` (701.19a-b): asked among the cards with the quality, each card once, a search with nothing to find shuffled or declined | proved, 198 s |
| `finish_library_search` (701.19a, 614.1c, 302.6) | proved, 133 s, once defect 12 was fixed |
| `arrives_tapped` (614.1c), a change of control, adding mana to a pool | proved, 49 to 65 s each |
| the creature groups a card's effect names ("creatures you control", "your opponents control", "creature tokens you control", "with flying your opponents control", "as long as you control a Human", "as long as an opponent controls a Human"), for a card and a token | proved, 32 to 65 s each |
| `replacement::for_entering` on a board printing no replacement (614.1) | proved, 39 s |
| `move_object`, `move_object_under_control` (400.7): a card from each zone it can leave into each it can go to, each pair its own harness | proved, 45 to 251 s each |
| `create_tokens_with_subtypes`, `create_token_copy`, `create_token_with_defined_pt` (111.2-4, 707.2, 604.3), as many as the replacement layer gives (614.1b) | proved, 51 to 251 s each |
| `apply_transform` (701.28), and so `transform_quietly`, which it calls and logs around: exactly a double-faced permanent turned over, not a token or a copy by way of a one-faced card (701.28c), nor one transformed since the ability transforming it went on the stack (701.28f); and the rule for a trigger, its own or another permanent's, delayed or not, stamped with the count now, an older one or none: only its own, not delayed, stamped before the last transform, transforms nothing ("isn't a delayed triggered ability", #987) | proved, 89 s and, the trigger's, 92 s |
| `store_ids`, `load_ids`, `clear_ids`: the lists a card keeps while it waits on an answer read back as stored, every other entry kept | proved, 9 to 91 s each |
| `payment_plan_with_sources`, `pay_cost_with_sources` (118.3, 302.6, 605.3a, 608.2g): with nothing to tap, and with a Forest, its plan, its payment and the Forest's activation, for six costs, either player and each pool of up to two of white, green and colourless | proved, 106 to 641 s each |
| a mana ability activated as a tap plan's entry is run (`activate_mana_source_reserving`, 605.3a, 302.6, 118.3), against the planner's own step (`mana::tap_step`): for a basic land, a dual's second colour, Sol Ring, a utility land, Shimmering Grotto's `{C}` and its filter paid out of the pool around the cost, a mana creature and Deranged Assistant, the pool left exactly as the step leaves it, the source tapped and both announced, nothing else changed but the Assistant's mill; a source tapped, a creature summoning sick and a filter with nothing to pay for it changing nothing; each for either player as the controller | proved, 24 harnesses, 132 to 322 s each at 3.5 GB (at 782c4ed) |
| what each until-end-of-turn effect a card adds does (611.2a): one or two power and toughness changes on a creature (613.4c), each keyword granted (613.1f), defender lost, can't block (509.1a) | proved, 24 to 45 s each |
| the cleanup step's end of them (514.2, `end_until_end_of_turn_effects`), each creature whose control one changed going to whom the effects still in force give it (613.7a, 110.2a) | proved, 226 to 402 s each, at 7 GB but for two changes of the second creature's, which took 12 |
| a creature's power and toughness from a setting, counters and a change at once (613.4b-c, 613.7a), none, one or two settings | proved, 48 to 75 s each |
| what each static effect a card prints does once it reaches a creature, beyond those above: can't be blocked (509.1b), Ghostly Possession's prevention, Unbreathing Horde's, Inquisitor's Flail's doubling (615.1, 614.1a) | proved, 34 to 157 s each |
| the scopes the Curses ("creatures enchanted player controls"), Instigator Gang ("attacking creatures you control") and the lords ("Vampire", "Werewolf", "Human", "Werewolf and Wolf creatures you control") print, for a card and a token | proved, 26 to 57 s each |
| Heartless Summoning's reduction and Rooftop Storm's alternative cost (601.2f, 118.9), each the card itself | proved, 28 to 35 s each |
| a mana cost as it is built (`ManaCost::new`, #978): every symbol but a `{0}` kept, in order, a cost of `{0}`s alone kept as it is, and the mana value unchanged, for costs of up to three symbols of every kind, a harness per length | proved, 2 to 4 s each |
| "you may pay" (`helpers::ask_may_pay`, 117.3, #940), whether the cost can be paid any answer: asked yes or no by the source, in the tap plan's words, exactly when it can be, and otherwise nothing asked or changed | proved, 50 s |
| who may block whom (`can_block_attacker`, 509.1a-b, 702.9b, 702.13b, 702.16b), every answer to every question it asks | proved, 30 s |
| the trigger collector (`triggers::collect::for_event`, 603.2, 603.3b): each kind of event a card in the pool has a trigger on (entering, dying, a step's beginning, a spell cast, attacking, blocking and being blocked, combat damage to a player or a creature, damage to a player, leaving, a card milled) raises the abilities it should, from the objects and under the controllers it should, exactly when their conditions hold, filed with the active player's or the other's | proved, 37 to 168 s each |
| a delayed trigger, the exile of Geist of Saint Traft's token (603.7a, 603.7d, 400.7): the end of combat puts it on the stack once, from Geist, under its controller, and it exiles the token if the token is still on the battlefield | proved, 85 to 159 s each at 3.5 GB (at 782c4ed; out of memory there at 156d7ad, where the merge named a trigger's targets for its log line eagerly) |
| casting a spell (`cast_spell`, 601.2a-i): with the targets it names legal (the check of a submitted declaration any answer), it moves to the stack holding exactly those targets and the zone it was cast from, goes on top, and its casting is the one event; with them not, it is refused before anything is done; for a spell that costs nothing, the mana kept back for the rest of the hand standing in as none | proved, 222 s |
| activating an ability (`activate_ability`, 602.2a-b): with the targets it names legal, its source tapped for its cost, it goes on top of the stack from that source, as the ability of that index of the card granting it, holding exactly those targets and the requirement they were chosen under, under the player who activated it; with them not, it is refused before anything is done | proved, 162 s |
| activating a loyalty ability (`activate_loyalty_ability`, 606.3-5): with the targets it names legal, its cost the loyalty it changes by (counters on for a plus, off for a minus, none for a zero), the planeswalker marked as having activated one this turn, and the ability on top of the stack from it, flagged a loyalty ability, holding exactly those targets and their requirement, under its activator; with them not, refused before anything is done | proved, 77 to 104 s each |
| declaring attackers (`declare_attackers`, 508.1): the attack recorded (`combat::declare_attackers`) is exactly the creatures declared, each with no blockers yet, tapped but those with vigilance, in order, stamped as having attacked this turn, and listed once each in the attack's event; and the declaration records who attacks, the creatures named first, then those forced; and who attacks (`who_attacks`, 508.1a, 508.1d): the named ones that may attack, each once, in the order named, then those that must, in the board's order | proved, 21 to 210 s each, who attacks once its lists were filled in loops (the declaration of none alone at 12 GB) |
| a trigger going on the stack (`push_one_pending_trigger`, 603.3c-d): one that does not target as it is; with no target allowed not at all; with one holding it; with two, waiting at the front of its controller's queue while its controller chooses between exactly those, the answer (`AttachTargetToPendingTrigger`) putting it on the stack holding the target chosen | proved, 30 to 73 s each |
| a spell's or ability's resolution (`resolve_top_of_stack`, 608.2b, 602.2a, 606.5, 400.7): with targets none of which is legal it does nothing, and otherwise the card's resolution is handed each target as chosen or as illegal, a target that has changed zones since it went on the stack illegal, as a new object; a spell leaves the stack, its card's resolution handed the targets it was cast with and their zone changes then, read off it (`resolve_spell`), and re-checked (`resolve_spell_as`), each half its own proof; each target gone and back or not, its count recorded or, as in a saved game made before counts were, not | proved, 44 to 106 s each |
| what `matches_filter` makes of each creature filter a card prints (non-Human, neither Vampire nor Zombie, flying or reach, non-Human creature, Wolf or Werewolf), and what the effects printing them do with it: "can't be blocked except by" (509.1b), protection until end of turn (702.16), Moonmist's prevention (615.1) | proved, 21 to 146 s each; Stromkirk Noble's restriction, the same shape as Blazing Torch's, ran out of memory at 7 GB beside four other runs and is run again alone |
| a triggered ability's resolution (`triggers::resolve_next_trigger`, 608.2b, 113.7a, 603.4, 603.3a, 400.7): with its one target no longer legal (changed zones since it went on the stack, its requirement, protection or hexproof, the card's restriction) it does nothing; otherwise the hook for its event, and only that one, is called once on its source, with the event's data and the target as chosen, while the ability is the one resolving, as from the face it triggered from, and with the player it triggered for as the controller `ability_controller` reads, whoever controls its source now; and nothing is left resolving after, nor a controller to read; for each kind of event the pool's triggers are on, and the requirement it is re-checked under the one its card prints (`target_requirement`) | proved, 12 to 58 s each, but a creature's dying, out of memory even at 12 GB. Not for the dead creature's lists: with them empty and every value of the event fixed, and with the trigger kept for the view copied without them (352,000 steps down to 319,000), the solver still runs out at 3.5 GB, where every other kind proves in 3 s of solving |
| flashback granted until end of turn (`graveyard_cast_costs`, 702.34a), as Snapcaster Mage and Past in Flames add it: a card in its owner's graveyard may be cast from there at each cost an effect on it gives, each once however many give it, and an effect on another card gives it nothing; which of the turn's effects are on a card (`granted_flashback_costs`), each the effect's own cost in the order granted, its own proof | proved, 19 to 64 s each |
| state-based actions (`check_state_based_actions`, 704.3): checked pass after pass until one does nothing or stops for a state trigger or a legend-rule question, and not at all in the middle of a resolution but for the choice of what a permanent enters as a copy of (614.12b); a pass (`one_pass`) takes each action once, in order, the creatures removed exactly those decided | proved, 11 and 19 s |
| the actions of a pass, each its own proof: players losing for their life or their draws (704.5a-b); which creatures die (704.5f-h: the kernel's rule asked of each, of a toughness from the layers or the object's own) and what is done to them (to the graveyard for toughness, which nothing stops; left if indestructible as the pass began; regenerated with a shield; destroyed otherwise); an attachment's legality (704.5m-n, 301.5, 303.4, 702.5d, 702.16c-d, each enchant) and an illegal Equipment coming off what it is on, an illegal Aura to the graveyard; +1/+1 and -1/-1 counters annihilating (704.5q); the first state trigger whose condition holds (603.8); a planeswalker with no loyalty to the graveyard (704.5i); the legend rule's question (704.5j) and its answer (the rest to the graveyard, each death captured); a token elsewhere ceasing to exist (704.5d), and an object ceasing to exist gone from the board and every library; the game ending with one player left or none (104.2a); and control effects ending with their condition (611.2b, 613.7a) | proved, 17 to 188 s each: the control effects' once the engine read the effects that end where they lie, and the legend rule's question (alone at 7 GB) once its groups were read so too |
| declaring blockers (509.1-2): which of the blocks submitted stand (the board read into the kernel's terms, each pair allowed exactly when the attacker attacks, the blocker is the defending player's and `can_block_attacker` lets it, each attacker's fewest blockers `minimum_blockers`; exactly what the kernel accepts recorded, in order); the blocks recorded (each attacker's blockers in order, blocked for the combat, a creature a blocking creature exactly when declared on an attacker attacking (509.1g, #983), the one event listing them); the damage assignment order announced (509.2: one blocker left placed without asking, the first attacker with two or more asked of, nothing after it placed); and the declaration as a whole (judged, the prompt answered, then the order announced) | proved: which blocks stand, 38 to 138 s each, and their recording; the order's announcement, six harnesses, 33 to 78 s each. The standing of no blocks and the six announcements ran out of memory even at 12 GB until two traps in their own boards were found: a declaration handed over as an array of no length (a slice of an object of no size, which CBMC follows as a pointer to anything; now an array of room, `cards::in_room`), and attackers made in a loop over an array of slices by value, whose slices, read back out of the iterator, the solver could not follow, so each object made after might have replaced another (now made one by one) |
| a creature removed from combat (`remove_from_combat`, 506.4): out of every part of the combat, and every other creature where it was, so an attacker it blocked stays blocked (509.2) and a creature that blocked it stays a blocking creature, blocking nothing (510.1d, #983); a harness per combat and creature removed (with either open the combat's sets were built or shifted every way, out of 5 GB) | proved, eight harnesses, 29 to 78 s each |
| the turn-based actions of each step (502-514): the untap step's untapping, summoning sickness and land play; the draw; the declarations asked; a combat damage step's first-strike and regular halves (510.4-5); the end of combat step leaving the creatures in combat (511.3); priority in the steps with none of their own; the cleanup's discard first (514.1), and the rest of it after (514.2-3a); the discard that answers it; and stepping on as the turn kernel says, with combat ending as the end of combat step ends | proved, 20 to 203 s each |
| one combat damage step's damage (510.1-510.5, 702.4, 702.7, 702.19b-c): which creatures deal damage in the step, to whom and how much, the division stood in by any division the kernel is proved to give; and the division question the step asks first (510.1c-d) | proved, 17 to 317 s each: the blocked attackers' once a damage step read its blockers where they lie and the recorder wrote each slot by its index; three of a first-strike step's, run alone at 7 GB, stopped by Kani, not by any assertion, on a copy into a list allocated on one path and not another (`memcpy destination region writeable`) |
| what a player with priority is offered: the timing a card may be cast at, from a hand or a graveyard (117.1a, 307.1, 702.8a), every set of types; their loyalty abilities and land plays (606.3-4, 305.1-3); what restricts an activated ability beyond its mana (602.5b, 601.2h, 302.6) | proved, 4 to 644 s each, the loyalty abilities a harness for each player |
| answering a "choose a target" question (601.2c, 608.2: an offered target taken, an unoffered one or a refused decline leaving the question as it was) and what a token entering attacking attacks (508.4) | proved, 74 to 107 s each |
| the spells cast in a turn (601.2i), what a Werewolf's "last turn" reads: casting a spell counts it to its caster, the other player's count as it was; leaving the cleanup step makes this turn's counts last turn's and starts the next at none, a repeated cleanup step keeping both | proved, 82 and 264 s |
| what an ability's costs leave its resolution (602.2b via 601.2h, 608.2h): the costs paid in order (the {T}, the creatures a cost taps, the counters, then the sacrifice), which creature was sacrificed and its toughness read before it was, a once-each-turn ability marked; the sacrifices offered (each creature the activator controls, but the source for "another", no offer with none); the ability put on the stack carrying them and its source's transforms; and its resolution reading them back, and its activator as its controller, whatever another ability left | proved: the sacrifices offered, the ability put on the stack and its resolution, 25 to 49 s each; the costs paid, a harness for each kind of sacrifice, 162 to 200 s each, once the log line for the creatures a cost taps (its names joined over a list of open length, half of what symbolic execution did) was moved into a function of its own (`tapped_for_cost_line`) the proof stands in for; out of memory even at 12 GB before |
| a spell's resolution's end (608.2n, 608.3, 306.5b, 702.34a, 701.5a): a card with no resolution of its own onto the battlefield if a permanent, legendary as printed, a planeswalker with its loyalty or X in counters; a spell done with into exile if cast with flashback, the graveyard otherwise, resolved or countered; moved off the stack unless its resolution put it somewhere, left resolving while a question it asked waits | proved, 14 to 34 s each |
| who has priority after each action the game loop plays (`hand_on_priority`, 117.3-117.4): a pass hands it on, ends the step, or resolves the top of the stack and gives it to the active player, as the turn kernel says; a declaration gives it to the active player; an answer to the caster if part of a cast, the active player otherwise; anything else leaves it | proved, 13 to 21 s each |
| the answers to questions asked mid-resolution or mid-cast (`resolve_choice`), each reply its own harness: the creatures a cost taps (601.2h), a set of objects for an effect (Curse of Oblivion's), a library search's card (701.19a-b, defect 21), a yes or no and a card type handed to the card that asked, which damage effect applies first (616.1), an X funded for an ability or a spell (601.2b, 601.2f-h, 107.3), a cast's exile cost and its "up to N" targets (601.2b-c); each accepted exactly as the rules allow, and anything else refused with the question standing (or, at cast time, the cast cancelled with nothing paid) | the yes or no, the card type, the library search, the damage effect, the X of an ability and of a spell (the cast's log line stood in for), the tap replies, the set of objects (alone at 7 GB) and four of the "up to N" targets (at 12) proved, 34 to 193 s each; the two "up to N" answers that accept a target failed on their list of targets growing, and are run again since it is made with room for them (43f4fe1); the exile cost's six, 78 to 101 s each, once the check for a card named twice compared each with those before it rather than sorting a copy (out of time at 1800 s each before) |
| combat's two answers: a blocker's place in the damage assignment order, one at a time or the whole order (509.2), and the damage one blocker is assigned (510.1c-d) | proved: a blocker's place one at a time and the damage assigned, 65 to 84 s each; the whole order's four, 79 to 96 s each, once the log line of the order completed (`log_completed_order`, its names joined, which a blocker placed alone never reaches) was stood in for; out of memory even at 12 GB before |
| abilities that triggered together going on the stack (`process_pending_trigger_pushes`, 603.3b, 101.4): the active player's first, each player's own in the order they choose, asked when two can be told apart and are not yet ordered; and the answer, a whole order or one trigger to put on now | the five of the queue proved, 42 to 51 s each, once each case's queue was built by a plain loop and not through `flatten` over a table (655 thousand steps and out of memory at 3 GB, 226 thousand so); the seven answers out of memory in symbolic execution at 3.5 GB, the whole order's after 940 to 990 s. The whole order is put back in place by swaps now, where lifting the triggers into `Option`s ran a crate of that shape alone out of memory; the answers by index share none of that code, so what is left is in what every answer of the question shares, and the seven are run alone at 7 and 12 GB. At 7 GB all seven still run out of memory (the whole order's after 900 to 1,020 s and 1.37 million steps, those by index in symbolic execution after 215 s). What they share is not the dispatch: an answer of the wrong kind to the same question, through the same `resolve_choice`, proves in 69 s (284 thousand steps). Nor is it the log lines: `std::fmt::format` stubbed out, the whole order still takes 1.37 million steps, and reading the prompt's positions by a plain loop does not save the answer by index either. It is in the two arms' own work on the queue |
| a spell's additional costs paid as named (601.2b, 601.2h, 118.3): a sacrifice made once, the creature cards named for an exile exiled in order, and an exile of X exiling the cards named, or as many as there are for an X past them, and counting them | proved, 57 to 89 s each |
| a permanent entering as a copy (614.1c, 707.9): asked among the creatures there, those it may copy pruned to those still there, and the choice recorded finishing the entry under its owner or the controller held for it | proved, 30 to 63 s each |
| the check of a submitted cast's or activation's targets (`targets_are_legal`, 601.2c, 602.2b) for each shape of requirement: none, one, up to two, and a modal one, each target named legal under the requirement (defect 22) | proved, 13 to 16 s each: the modal one as the choice it is, each mode asked in turn with the targets as named until one says yes, what a mode says the other three proofs' (`legal_under_mode`, split out of `targets_are_legal` for it); followed through each mode it ran out of memory at 12 GB |

A card proof ends at what the card leaves on the board: Giant Growth's
says the card adds a `TemporaryEffect::ModifyPT` of +3/+3 on its target
and changes nothing else, Heartless Summoning's that it prints a
`ReduceCost` of two for creature spells, Geist of Saint Traft's that its
attack leaves a delayed exile. The rows from the until-end-of-turn
effects down are what the engine makes of each such thing: for the kinds
they name, the question that reads it answers what the card's words say.
`ModifyPTWhileSourceInPlay`, the one kind no card adds, has none. Four
engine lists and reads became functions for this, unchanged:
`end_until_end_of_turn_effects` from the cleanup step,
`names_that_cant_be_cast` from the legal actions, `resolve_spell_as`
from a spell's resolution and `granted_flashback_costs` from the
graveyard's casts; the others the entries above name were changed to
read what they read in place. Writing the triggered ability's proof
set the three re-checks of a target on resolution side by side, and
two did not ask about protection: defects 14 and 15 below.

Three of these held only once the function asked what the rule says:
`untap` untapped a card off the battlefield, `discard_card` discarded a
permanent, and `attach` left an Aura attached to the player it came off;
no game reached any of them, every caller offering only what the rule
allows, and each has a regular test besides.

Where it stands. Every harness was run at one commit, d461ae7 (1,194
of them then), and those whose code changed after it were run again at
the commit that changed them, with what each change reached listed
beside the results. The 43 that had run out of memory at 7 GB were run
alone at 12, and those that did there too were restructured, as the
entries above say: a constant in each harness for what the solver had
left open (a library's length, a graveyard's owners, a planeswalker
board's shapes, Make a Wish's objects), a harness per assertion where
the parts were cheaper apart, room for exactly what a list is pushed,
an engine action proved once and stood in for, and two cheaper engine
lists (`creatures_controlled_by`, Grimoire of the Dead's scan), the
engine's tests passing after each. Every harness those changes reached
was run again.

Every harness, 2,042 of them, was then run in one pass, the results in
scratch files the ledger is written from: at 3.5 GB three at a time,
then each that ran out of memory alone at 7 GB, then at 12. The pass
began at f6a9869 and moved to 65fde47 when the recorder's write was
found to cost proofs that record several actions far more than it saved
(below); 65fde47 writes the same value into the same slot, so the 1,212
harnesses that had passed at f6a9869 were kept, but the casting proof,
whose engine code changed between them. The engine changes after it, a
cast's log line (e60310f, 2a751bf) and an "up to N" answer's list of
targets (43f4fe1), are taken only by the casting proof, the spell X
proofs and the six "up to N" answers (the other proofs that call
`resolve_choice` give an answer of another kind, and cannot take that
arm), which are run at the commit that changed them. Every card harness
passes: 989 over the 248 cards and four tables of cards, 14 of them
alone at 12 GB (`reports/kani-card-ledger.md`, card by card). Of the
1,024 harnesses of engine actions, 984 pass, and the 29 others, of the
map and set types proofs keep the board in (`vec_map`, `kind_map`) and
of mana payment, all do. The 40 left are named in the rows above: the
costs an ability's activation pays, the damage assignment order's
announcement, a blocker order given whole, the order of triggers and the
exile cost's answers, a modal requirement's check and a dying trigger's
resolution, out of memory even at 12 GB or out of time in symbolic
execution; three blocked attackers' first-strike steps, stopped by Kani
on a copy into a list allocated on one path and not another; and the two
"up to N" answers run again at 43f4fe1.

That pass was made with the Kani defect "What is trusted" describes
present in it. The three first-strike steps were its failures: their
"list allocated on one path" was a `Vec::new()` whose capacity was read
from the proof's own counter. A scan of one harness from each of the
104 modules of the proofs found 34 reading a constant through a static
(the card library's `LISTINGS_HANDED` and `ACTED`, and the action
proofs' own counters), four of them through `Vec::new()`, and those four
had passed. So no result of that pass is kept: every harness is run
again, with the proofs' statics `Held` and the scan before each, and the
counts above are replaced as that run finishes. It was begun at f288a21,
then at f00e4ef once master was merged in (its engine changes reach the
proofs), and is made at 156d7ad: the scan first counted the memory Kani
lays out for what a constant pointer points at as a static (41 harnesses
recorded ERROR for nothing), and a `Held` laid out with its value eight
bytes in lost what was written there, so that the offer proofs of who
may attack and block failed (the value is now first, where it was). Built so,
the first-strike step of a blocked attacker and its blocker passes (512
s; 613 thousand steps of symbolic execution where the shared constant
cost 1.5 million). Three changes after 156d7ad reach proofs the run
holds: the autotap planner's (the payment proofs, `proofs::actions::
paying`, 226 harnesses, are run on it instead, and not at 156d7ad), the
check of a submitted cast's targets split per mode (its four proofs run
again), and the own-trigger transform proof's trigger built through the
collector (run again, 103 s); the activation proofs are new. And 18
harnesses that proved at the first try in the run before ran out of
memory at 3.5 GB here, the delayed exile of Geist of Saint Traft's token
among them: measured at each commit between, the master merge took that
one from 271 thousand steps to 347, all of it in naming a trigger's
targets for its log line, which the merge's exile trigger now carries
and which were built as `format!`'s argument whether or not a line was
written. Named only as the line is written (1ea7067), it takes 267
thousand steps and 0.5 million clauses where it took 1.3 million, and
proves at 2.5 GB; those 18 run again on that code.

The run moved to 782c4ed after 896 harnesses, for the rest and for every
harness of a module whose code had changed since 156d7ad: card proofs at
156d7ad were running a third slower than in the run before, with the
same steps (the machine, not the code), and the rest of the pass is then
the code being merged. Of the 2,080 harnesses, 1,885 proved at 3.5 GB
(the eight delayed exiles among them, 85 to 159 s) and 195 ran out of
memory there; each of those is run alone at 7 GB and then at 12, and
`scripts/kani/expected-open.txt` lists the ones that do not prove. With
the 7 GB pass done, the 12 GB pass under way, and what defects 34 to 36
touch run again at their fixes, the merged results (a later result for a
harness replacing an earlier one) leave 70 of the 2,087 open, from 171:
967 of the 990 card harnesses prove, and of the 23 that do not, 15 are
still queued at 12 GB and 8, cards whose hooks defect 34 changed, ran
out at 3.5 GB where they proved alone at 7 before and are queued alone
at 7 again, with the trigger collector's 19 (the same pattern) and the
mana abilities' 12. No
harness failed but Mentor of the Meek's two may-pay proofs, which still
asserted the question is always asked, where master's #940 asks it only
when a payment can be planned: their proof now holds the question to the
planner's stand-in (asked exactly when it finds a plan, nothing done when
not), and both prove at 321bcd4.

The engine paths a card proof stands in for or takes for granted were
then walked function by function for any with no proof: the answers to
the questions a resolution or a cast asks (each reply its own harness),
the order triggers go on the stack, the steps of the game loop around
each action (what is done before a player receives priority, and who
receives it after), where a spell goes once done with and what its
default resolution does, an ability's costs and the facts they leave its
resolution, the check of a submitted cast's targets, a spell's
additional costs, the spells counted in a turn for a Werewolf's "last
turn", and a permanent entering as a copy. Four engine functions were
split out of the game loop and the offer and answer code for it,
unchanged (`hand_on_priority`, `before_priority`, `sacrifices_offered`,
and the legend rule's first group found in one pass), and `who_attacks`
fills its lists in loops; every crate's tests pass after each. Two
defects were found on the way (21 and 22 below). Three engine lists were
found read at positions the board decides, so that every object named
from them was any object: the control effects that end, filtered into a
list of their own before each permanent was looked up; the legend rule's
groups, filtered and grouped from the battlefield; and a blocked
attacker's blockers still in combat, filtered from its damage assignment
order before each was dealt with. Each is now read where it lies, one
entry per effect, object or blocker, the same effects ending, the same
legends asked about and the same damage dealt, and the engine's tests
pass. Three causes of cost were found in the proofs' own machinery: the
recorder assigned each action into its slot, and an assignment drops
what the slot held, which where the board decides how many actions came
before was a drop of every kind of action at every place (44% of what a
blocked attacker's damage followed); written through a pointer to the
slot instead, which skips the drop, the place the board decides cost
proofs that record several actions far more (Divine Reckoning's took
eleven times as long, past half an hour), and now each slot is written
by its own index in one arm of a match on the count, cheap for both; the
set type the combat's sets are kept in under Kani boxed nothing for each
entry, a dangling pointer that Kani's pointer checks spent gigabytes on
(now the keys in order, proved an ordered set); and kani-bounded found a
table's harnesses only for tables it was told the names of, so 73
harnesses written in new tables were passed over without a word until it
found any macro whose definition writes a proof, and said which
requested names select none. Two proofs' own boards were made cheaper
too: the loyalty offers are a harness for each player, and the set's
proofs reserve their room as the map's do. Each proof that still runs
out of memory or time is named in its row above, with what is known of
why. The run at 6b80c8a was overtaken by these changes, which reach
every proof (the recorder's write, every card proof's), and was stopped
with 213 harnesses left; the pass above replaced it.

A counterexample can be of a board no game reaches, which is then the
board's error and not the card's: the board the counted creatures are
proved over gave each card a power, a creature or not, and the engine
takes an object with a power for a creature, so Boneyard Wurm counted a
non-creature card. A noncreature has no power (CR 208.3); the board now
gives one only to a creature, and the proof holds. And a failure can be
the proof's own: Memory's Journey's built its two targets in a list with
room for one, and failed on the list growing, whatever the card did; and
holding the ids of new tokens to eight failed Army of the Damned's
thirteen, until they had room of their own; and Grimgrin, Corpse-Born's
expected targets read each object's own card types, which a card's
object does not carry (they are printed), so with the player choosing
Grimgrin's opponent, the engine rightly offered Grimgrin itself and the
proof failed on it, at seven gigabytes, until it counted Grimgrin a
creature as printed.

### The whole engine under libFuzzer

`fuzz/fuzz_targets/game.rs`. The first campaign ran for 118 minutes
across three workers: 22,906 whole games, a corpus of 3,515 inputs reaching
27,178 code edges (still climbing slowly when it stopped), and no finding:
no invariant broken, no panic, no overflow, no stall. The nightly job keeps the corpus from night to night,
so each night starts from everything the last one reached.

### Defects found

| # | defect | how it was found | reach |
|---|---|---|---|
| 1 | a payment that failed still spent mana: `{W}{W}` out of a lone `{W}` emptied the pool | writing the payment spec | latent: the only costed mana ability is generic |
| 2 | paying around a reserve took all of one type before the next, spending the hand's `{W}` beside a spare `{U}` | writing the spec; Kani found a counterexample on its own in 13 s | whenever mana floats before a payment, and in the autotap planner's judgement of plans |
| 3 | the spare was counted before the cost paid its own pips | writing the spec | as 2 |
| 4 | combat damage was divided silently whenever the power did not exceed the blockers' total lethal, though 4 power on two 2-toughness blockers may go 2-2, 3-1 or 4-0 | the completeness property; restoring the old rule makes the proof fail | over the same 608 seeded fuzz games the question came up 273 times, where it had come up 143 |
| 5 | X paid from floating mana took all of one type first, the same as 2 | proving the allocator by reuse | every seat's X answer: CLI, fuzzer, page |
| 6 | the autotap planner gave each coloured pip the best free source in turn and never revisited a choice, so three duals that pay `{W}{U}{B}` only in a cycle got no plan | the exhaustive small-board check, at three sources | latent: no card in the pool costs three colours |
| 7 | an ability transformed its own source again after the source had transformed since it went on the stack (CR 701.28f): Moonmist in response to a werewolf's upkeep trigger was undone by the trigger, a second Bloodline Keeper activation held under the first turned Lord of Lineage back, and held Ludevic's Test Subject activations flipped the Abomination back. A test asserted the first | writing the reference's "transform this creature" for Thraben Sentry, from 701.28f | any response that transforms a permanent whose own transform ability is on the stack |
| 8 | Daybreak Ranger chose between its faces' abilities, both index 0, by the face up at resolution, so Moonmist in response to "deal 2 damage to target creature with flying" made Nightfall Predator fight it; Homicidal Brute's end-step ability did nothing at all once turned back in response | the check of every double-faced ability with its source transformed in response | Moonmist, or any transform, in response to either |
| 9 | Unbreathing Horde entered with a counter for each Zombie of the player who last controlled it, not of the player it was entering under: returned by an opponent's Grimoire of the Dead, it counted its owner's Zombies | writing its Kani card proof, whose replacement is offered the Horde entering under either player | any effect that puts the Horde onto the battlefield under a player other than its last controller |
| 10 | Garruk, the Veil-Cursed's −1 sacrificed the activator's creature and then searched the library of whoever controlled Garruk as it resolved, not the activator's (CR 602.2a) | writing its Kani card proof, which separates the activator from the controller | a Garruk whose control changes with the ability on the stack |
| 11 | an ability an Aura grants was offered only to a player controlling the Aura, so a Skeletal Grimace on an opponent's creature gave its regeneration to no one (CR 602.2: the enchanted creature's controller activates it) | reviewing Skeletal Grimace for its Kani card proof | any Skeletal Grimace on a creature its controller does not control |
| 12 | finishing a library search cleared the summoning sickness of whatever it put onto the battlefield, so a creature card a search put there could have attacked, or paid a {T} cost, the turn it arrived (CR 302.6) | the Kani proof of `finish_library_search`, which says nothing is done to the card but what entering does | latent: the pool's one search onto the battlefield is Ghost Quarter's, for a basic land |
| 13 | a replacement that answered a count of none still created one token: the copies were made in a loop of one fewer and the last outside it (CR 614.1b: the count the replacement answers is the count created) | the Kani proof of token creation, which takes every answer the replacement layer may give | latent: the pool's one token replacement, Parallel Lives, only doubles |
| 14 | a triggered ability's target was re-checked on resolution for its requirement, hexproof and the card's restriction, never for protection (CR 608.2b, 702.16b), so Spare from Evil in response to Reaper from the Abyss's end-step ability, or Grimgrin's attack, gave the target protection from non-Human creatures and the Demon destroyed it anyway | setting the three re-checks side by side to write the trigger resolution proof: an activated ability's asked `can_be_targeted_by` | any targeted trigger of a non-Human creature's at a creature given Spare from Evil in response |
| 15 | a spell's re-check asked the same two halves and not protection, so a spell whose only target gained protection from it resolved, its target marked illegal, where the rules counter it | the same comparison | latent: every protection the pool grants is from creatures or creature types, which no spell here has |
| 16 | the check of the targets a submitted cast or activation names (CR 601.2c, 602.2b), there because neither client submits a whole offered action, asked the same two halves and not protection, so a creature with protection from the source, which no list offers, could be named, and the spell went onto the stack | reading the check while scoping the proof of casting | an action assembled by hand, a test or a replay: the clients build theirs from the offered lists |
| 17 | a loyalty activation's submitted targets were not checked at all, so a loyalty ability could be put on the stack at a target no list offered: Garruk Relentless's 0 at an opponent's creature with hexproof | the same review, of the third activation path | as 16 |
| 18 | the cleanup step discarded to hand size last, after damage was removed and "until end of turn" effects had ended and after the state-based check (CR 514.1 puts it first), and asked for the discard with the active player holding priority: through `submit_action` they kept it after answering, and a Giant Growth cast then lasted into the next turn | splitting the turn-based actions into a proof per step, reading the cleanup's against 514.1-514.3 | every cleanup step with more than seven cards in the active player's hand: a creature held up until end of turn died before the discard, not after |
| 19 | combat was cleared as the end of combat step began, where CR 511.3 removes creatures from combat as it ends: through the step nothing was attacking or blocking, so Instigator Gang's and Wildblood Pack's "attacking creatures get +N/+0" ended a step early and nothing could target an attacking or blocking creature then | the same split, the end of combat step's | every combat with an attacking creature's bonus, or a flash "attacking or blocking creature" target (Hollowhenge Spirit) in that step |
| 20 | a cast from a graveyard checked its timing with its own copy of the check, which left out artifacts and planeswalkers, so a card of either type that its text lets be cast from a graveyard was never offered | proving the timing once, for a hand and a graveyard alike | latent: the pool's one such card, Skaab Ruinator, is a creature |
| 21 | a library search's answer was taken on trust: the card named was found whether or not the search had offered it, so answering "search for a basic land card" with any card id found that card (CR 701.19a-b), as the looked-at choice once did | writing the proofs of each answer to a question asked mid-resolution, setting the three answers by card side by side: the other two check the card against what was offered | any seat that builds its answer rather than picking a row: the game loop admits any answer to a standing question and leaves its contents to `resolve_choice` (Traveler's Amulet, Caravan Vigil, Ghost Quarter, Garruk) |
| 22 | the check of a submitted cast's targets asked an "up to N" requirement's targets their requirement and the card's restriction and not whether the source may target them, the half defect 16 added to every other requirement's: a Feeling of Dread at a creature with protection from instants went onto the stack | writing the proof of that check, which `casting` and `activating` stood in for as any answer | as 16, for the pool's three "up to two target creatures" spells; latent but for a test, as every protection the pool grants is from creatures |
| 23 | the autotap planner counted each Deranged Assistant's ability as it is before any of the plan runs, so two over a one-card library were both planned for `{2}`: a cast was offered and then refused, and a resolving "you may pay {2}" was asked, and paying it tapped the first, milled the card and then could not pay (CR 701.17b, 118.3) | proving the planner sound for every board: asking what a step can change that the next step reads | two Deranged Assistants and one card left in the library |
| 24 | a greedy plan that did not pay was handed on as it was: with nothing in hand to keep payable it was never run, and with something it was run, found not to pay, and returned all the same | the same: reading every way out of `keep_the_hand_castable` | latent: no board of up to three sources reaches it |
| 25 | the auto-pass gate's count took the most of any one of a source's produced entries for a type, so an ability listing a type twice (`{C}` and `{C}`) counted once, and a cost it pays was ruled out | proving the count necessary in Verus, which asks what an ability adds of each type | latent: every mana ability in the pool adds one kind (`proofs::cards::mana`) |
| 26 | a token copy of a card that chooses what it enters as was built onto the battlefield, where that choice was never asked: Back from the Brink copying an Evil Twin card made a 0/0 that died, though the token's copiable text is the card's, the choice among it (CR 614.12b, 707.2) | reading Back from the Brink in the `card_semantics` reference, whose token enters as a cast creature would, each way it can | Back from the Brink with Evil Twin in its controller's graveyard, and Cackling Counterpart on an Evil Twin that declined and lives (under Glorious Anthem, say) |
| 27 | the state-based actions read each dying creature's last known information just before its own move, after the moves of those dying with it, so a toughness that counts the creature cards in a graveyard counted one dying beside it, and indestructible was read after the creatures of no toughness had gone (CR 704.3, 608.2h) | Abattoir Ghoul in the `card_semantics` reference's every-fight run: attacking into a first-striking blocker under Wreath of Geists, both dying, it gained 4 for a blocker that died a 3/3 | Abattoir Ghoul's life, the one trigger that reads a dead creature's toughness, for a creature dying beside a creature card that counts towards it; and indestructible from a creature dying of no toughness in the same pass |
| 28 | a creature card that chooses what it enters as, returned with others at once (Grimoire of the Dead, Moldgraf Monstrosity), was asked once the others had entered, and offered them as copies, though Evil Twin's ruling allows only "a creature that's already on the battlefield" (CR 614.12b) | reading Grimoire of the Dead's return with an Evil Twin card in a graveyard into the `card_semantics` reference | an Evil Twin card in a graveyard that Grimoire of the Dead or Moldgraf Monstrosity returns from |
| 29 | Grimoire of the Dead's "They're black Zombies" was never given to the card whose entry waited on that choice: the return moved on past it, and it entered, as itself or a copy, neither black nor a Zombie (CR 613.1d-e) | the same | as 28, for Grimoire of the Dead |
| 30 | "Whenever this creature deals combat damage to a creature" was raised for combat damage to any object, a planeswalker among them: Creepy Doll attacking Garruk Relentless flipped its coin, and a won flip destroyed him (CR 510.1a-c) | the `card_semantics` reference's attacks on a planeswalker, whose Doll flips only for a creature it damages | Creepy Doll, the one card printing such a trigger, attacking an opponent's planeswalker and not blocked |
| 31 | a creature card milled while Undead Alchemist was on the battlefield set off nothing when the state-based actions took the Alchemist after the same step: the collector looked for watchers on the battlefield once they were done, not as the card was milled, though the damage collector beside it had looked back (CR 603.2) | the `card_semantics` reference's two attackers in the exhaustive run: Makeshift Mauler and Undead Alchemist attacking, Typhoid Rats blocking the Alchemist, Moon Heron milled and left in the graveyard with no Zombie made | Undead Alchemist killed in the combat damage step in which another Zombie's damage is milled instead |
| 32 | Curiosity's draw was not offered when the creature it enchants dealt damage to an opponent and died in the same step: its condition asked for the Aura on the battlefield when the triggers were collected, after the state-based actions had put it into the graveyard beside its creature (CR 603.10) | the `card_semantics` reference's Aura fights, once the reference read what combat damage sets off as it is dealt: Festerhide Boar with Curiosity trampling over Ambush Viper | a creature with Curiosity dying in the step in which it damages an opponent: a trampler, or one blocked by a first striker it also hits |
| 33 | a death watcher that left the battlefield in the same event as the creature it saw die raised its trigger under its owner, not the controller it had: a Falkenrath Noble its owner's opponent had taken, dying beside another creature, gave its owner the drain for that death (CR 603.3a, 608.2g) | reading the trigger collectors for the last-known-information slips defects 31 and 32 were | a creature-death watcher under its owner's opponent's control (Olivia Voldaren, Traitorous Blood) dying beside another creature. The face is still read off the moved object: a Thraben Militia dying so raises Thraben Sentry's trigger, which then does nothing |
| 34 | a triggered ability's "you" was its source's controller as it resolved, not the player who controlled the source as it triggered: Sturmgeist taken in response to its draw drew for the player who had taken it, and Murder of Crows asked them its "you may draw" (CR 603.3a). An activated ability's controller was already set for its resolution; a trigger's was never set, and 32 cards' trigger hooks read their source's controller | the `card_semantics` reference's borrowed attackers: a mutant giving a combat damage trigger to its source's owner went uncaught, because Sturmgeist's hook read its controller again as it resolved | a source changing controllers between its trigger and its resolution: Olivia Voldaren taking it in response (after making it a Vampire), or leaving the battlefield and handing back what she took |
| 35 | Tree of Redemption's "exchange your life total with this creature's toughness" exchanged the life of the Tree's controller as it resolved, not its activator's: a Tree taken in response exchanged the taker's life (CR 602.2a) | the same sweep of what each card's resolution reads as "you", made for defect 34 | the Tree changing controllers between activation and resolution, by Olivia Voldaren once it is a Vampire |
| 36 | a spell's, an activated ability's or a triggered ability's target that left its zone and came back before it resolved was still its target: an object id outlives the object's zone changes, and nothing compared the object with the one targeted, so a Giant Growth whose creature was sacrificed and returned in response pumped the creature that came back (CR 400.7, 608.2b) | reading the target re-checks beside the zone-change count the engine keeps for nothing but a staleness check no code made, after the source-gone sweep | a target returned to its zone at instant speed while what targets it waits: Grimoire of the Dead's activation after the creature is sacrificed or killed, or Moldgraf Monstrosity's random return |
| 37 | a player's view listed their own library in the order it lies, top first, so every seat was sent its next draw, and the browser page's library overlay, headed "order unknown", showed it (CR 401.2); the terminal counts the library by name and the LLM seat only looks names up in it | a test that a player's view is the same however each library is ordered and whatever the other player holds or draws (`mtg-engine/tests/view_hidden_information.rs`), written to check that hidden information stays hidden | every game, every seat: the list is now by name |
| 38 | the names of the cards a pending question chose among were put in both players' views, whoever was asked: while one player chose the card of their hand to discard (Liliana of the Veil, Frightful Delusion, Murder of Crows), looked at the top of their library (Delver of Secrets) or searched it, the other seat was sent those cards' names (CR 402.3, 701.16, 701.19) | the same test, with each kind of question put to the other player | every game where one player discards from a hand of two or more, looks or searches: the names now go to the player asked alone |

Each has a regression test that fails without its fix.

Known and not fixed:

- An ability's source that left the battlefield and came back before the
  ability resolved, reachable only through a return at instant speed
  (Grimoire of the Dead's activation, Moldgraf Monstrosity's random
  return), is a new object (CR 400.7), and what the ability does
  to "this creature" should do nothing; the engine does it to the object
  that came back, whose id is the old one's. Mikaeus, the Lunarch's
  "{T}: put a +1/+1 counter on Mikaeus", with Mikaeus sacrificed and
  returned in response, puts the counter on the Mikaeus that returned.
  Defect 36's record of each target's zone changes is the pattern a fix
  would follow, stamped for the source as `source_transforms` is (defect
  7); what it would have to reach is every effect a card applies to its
  own source, counters and until-end-of-turn effects among them, most of
  them written by the cards themselves.
- A death watcher's face is read off the dying object after its move has
  turned it face up: a Thraben Militia dying beside another creature, in
  any combat, raises Thraben Sentry's trigger, which then does nothing
  but sit on the stack (defect 33). No back face in the pool watches
  deaths, so no trigger is lost.

### What the proofs catch that the tests do not

`reports/verification-mutants.md`: 29 mutants of the payment and combat
kernels, and 14 of the funding kernel. Of the first 29 the proofs caught 24
and the tests 26, and each caught mutants the other missed; of the funding
14 the proof caught all and the tests 13, the one they missed changing an
output the engine did not read until it was asserted. The
proofs alone caught a combat change that asks about blockers out of order
and silently cuts an answer already given, which the exhaustive test cannot
see because the set of reachable divisions does not change. The tests alone
caught changes to which spare mana is spent first, a preference the proof
deliberately leaves open.

## Conventions

- A proof is named for its property, in words, like a test.
- A spec is written from the CR or the Oracle text, never by calling the
  function under proof, and its CR references are in its doc.
- Every bound is stated in the proof's doc and in this file, with the
  reason for it.
- A new proof is shown able to fail: at least one mutation of the code it
  covers must make it fail (and `kani::cover!` shows a Kani proof's
  interesting cases are reached).
- A defect a proof finds gets an ordinary regression test, and its fix goes
  in its own commit, ahead of the proof. That keeps the fix reviewable on
  its own, and keeps "make the proof pass" from quietly changing what the
  engine does, the reward hack the post warns about.
- A change to a spec function is a change to what "correct" means, and is
  reviewed as one.
