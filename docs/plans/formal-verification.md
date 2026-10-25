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
five defects in code all of the above had passed (see
[defects found](#defects-found)).

## Tools, and what they turned out to be able to do

**Verus** is the main tool. It proves Rust annotated with contracts
(`requires`, `ensures`, loop invariants) using the Z3 SMT solver, which
reasons about integers rather than bits, so a proof holds for every amount.
The proved code lives in the `mtg-kernel` crate, inside `verus!` blocks.
`cargo build` erases the contracts and compiles the rest as ordinary Rust,
so the engine runs the code that was proved, not a copy of it. Only
`mtg-kernel` depends on `vstd`. Both kernels verify in about a second, once
`vstd` is built. Verus has no specification for some of `std` (it rejected
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
  state (`GameState`, `ManaPool`, `CombatState`) is `BTreeMap`s.
- **CBMC's SMT backend** (`--z3`) crashes on Kani's output (`map::at`) as
  of Kani 0.68.

The pattern every proof so far follows: pull a rule out of the state into a
function over plain values (arrays, slices, small structs), prove that
function in `mtg-kernel`, and keep the layer that reads the state into it
and writes the result back small, with unit tests on it.

### Running the proofs

    scripts/prove.sh                   # Verus, then Kani: about four minutes
    scripts/prove.sh verus             # mtg-kernel only: seconds once vstd is built
    scripts/prove.sh kani [HARNESS...] # the Kani proofs, or the named ones
    scripts/kernel-mutants.py          # what the proofs and the tests each catch

The `proofs` workflow runs both provers on every push to master and every
pull request. The Verus release (0.2026.10.04), its Rust toolchain and the
Kani version (0.68.0) are pinned in `scripts/prove.sh`, and `vstd` in
`mtg-kernel/Cargo.toml` must match the Verus release. Run the Kani proofs
one at a time, as the script does: four at once ran a 15 GB machine out of
memory.

### What is trusted

A proof is a proof relative to what it assumes. Here that is:

- the **specification functions** (`pub open spec fn` in `mtg-kernel`, and
  the specs in `mtg-engine/src/proofs/`). These are what a person reviews
  against the CR; everything else is checked by machine;
- **Verus, Z3 and Kani/CBMC**, and `vstd`'s specifications of the `std`
  functions the kernels call (`min`, `clamp`, `saturating_sub`, `Vec::push`);
- **rustc**, compiling the same source for the game that Verus checked;
- the **glue** that is not proved: reading `ManaPool`'s `BTreeMap` into six
  counts and writing the result back, and mapping blockers' ids to
  positions. Each is a few lines with unit tests, and the combat mapping is
  also covered by the exhaustive division test.

## The tiers

### Tier 1: pure kernels against rules-derived specs

Pieces of the engine that are arithmetic or combinatorics over a few
values. Each gets a spec written from the CR, proved for every input.

| target | spec (CR) | status |
|---|---|---|
| paying a cost from the pool: `can_pay`, `auto_pay`, `auto_pay_reserving` | 107.4, 601.2h | **proved**, three defects fixed |
| combat damage division, `divide_combat_damage` | 510.1c-d, 702.19b, with 509.2's order | **proved** sound and complete, one defect fixed |
| X funding from floating mana, `allocate_for_x`'s pool step | 107.3, and the reserve property of payment | **proved** by reuse of `pay_around`, one defect fixed |
| autotap planner soundness: a plan it returns pays the cost | 601.2g-h, 605.3 | next; `autotap_brute_force.rs` is the sampled version |
| autotap planner completeness: if some set of sources can pay, it finds one | 601.2g | next |
| power and toughness from counters and effects (layer 7) | 613.4, 704.5q | planned; needs extracting from `state.rs` |
| lethal damage, deathtouch, and what state-based actions destroy | 704.5f-h, 702.2b | planned |
| the turn's step sequence and who gets priority | 500.1, 117.3-4 | planned |

### Tier 2: the whole engine, through the fuzz oracle

The fuzz oracle's properties (`check_core`, `check_settled`,
`check_transition`, `check_legal`) are already a formal statement of what a
coherent game is, reviewed and maintained. The Anthropic post on LLM-driven
verification (2026-10-22) turned each existing fuzz harness into a spec ("this
test always passes") rather than writing specs by hand. The same move here
gives the whole-engine theorem: **from any state a game can reach, no legal
action makes the engine panic or break an invariant.**

1. Write it as a coverage-guided fuzz target (`cargo fuzz`) to the rules of
   Theorem's verification skill. The input bytes decode, totally, to a
   sequence of choices. Every property aborts when it fails. Seed inputs
   reach every check, and each check is shown to fail under a one-line
   mutation. Counts and recursion taken from the input are bounded, and the
   input is the only source of randomness. This is useful the day it
   exists: the random seats do no coverage guidance.
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

### Tier 4: whole-game properties

That priority passing ends (CR 117.4), that the stack resolves last in,
first out (CR 405.5), that state-based actions reach a fixed point (CR
704.3). The first plan was a Lean model of the turn and stack machine. The
post argues for proving the code that runs rather than a translation of it
(a model can disagree with the code), and Verus has now proved real engine
code here, so the preference is: extract the turn-step and priority
functions into `mtg-kernel` and prove them there; a Lean model only for what
no extraction can reach, checked against the engine on fuzz traces.

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

### X funding from floating mana

`allocate_for_x`'s pool step is `pay_around` with an all-generic cost, so it
carries that proof: it spends exactly what X asks of the pool, and leaves
what the rest of the hand needs whenever the pool could pay both. The page
has its own copy of the allocator, held to the engine's answers by
`x-funding-cases.jsonl` (1,542 checks over 168 boards).

### Defects found

| # | defect | how it was found | reach |
|---|---|---|---|
| 1 | a payment that failed still spent mana: `{W}{W}` out of a lone `{W}` emptied the pool | writing the payment spec | latent: the only costed mana ability is generic |
| 2 | paying around a reserve took all of one type before the next, spending the hand's `{W}` beside a spare `{U}` | writing the spec; Kani found a counterexample on its own in 13 s | whenever mana floats before a payment, and in the autotap planner's judgement of plans |
| 3 | the spare was counted before the cost paid its own pips | writing the spec | as 2 |
| 4 | combat damage was divided silently whenever the power did not exceed the blockers' total lethal, though 4 power on two 2-toughness blockers may go 2-2, 3-1 or 4-0 | the completeness property; restoring the old rule makes the proof fail | over the same 608 seeded fuzz games the question came up 273 times, where it had come up 143 |
| 5 | X paid from floating mana took all of one type first, the same as 2 | proving the allocator by reuse | every seat's X answer: CLI, fuzzer, page |

Each has a regression test that fails without its fix.

### What the proofs catch that the tests do not

`reports/verification-mutants.md`: 29 mutants of the two kernels. The proofs
caught 24 and the tests 26, and each caught mutants the other missed. The
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
