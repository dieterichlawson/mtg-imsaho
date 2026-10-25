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
a full model of the CR would be a project on the scale of seL4 or CompCert:
many person-years. What can be built, starting now, is a growing set of
proved properties. Each one rules out a class of bug for good, and the
tests, the fuzzer, mutation testing and the playtests keep covering
everything the proofs do not reach yet.

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
fuzzer never reached. The first spec written for this effort turned up
three defects in code all of the above had passed (see
[mana payment](#mana-payment)).

## Tools, and what they turned out to be able to do

**Kani** (bounded model checking of the real Rust, through CBMC) is the
tool in use. It checks the code as compiled, so there is no second copy of
the engine to keep in step. A Kani proof is a theorem over every value of
its symbolic inputs, within stated bounds. Measured on this codebase, on
the first day:

- **Small pure functions over small values: fast.** The payment kernel's
  proofs take from under a second to three minutes each.
- **Sums of many numbers: the cost grows steeply with the values allowed.**
  CBMC reasons bit by bit. The reserve proof finishes in 40 seconds with at
  most 3 of each type of mana, and did not finish in nine minutes with at
  most 5. So a Kani proof over arithmetic is a proof for small amounts.
- **`BTreeMap`: out of reach.** One insert and lookup proves in five
  seconds; six inserts did not finish in five minutes. `GameState`,
  `ManaPool`, `CombatState` and most of the engine's state are
  `BTreeMap`s, so Kani cannot reason about them directly.
- **CBMC's SMT backend** (`--z3`), which would reason about integers
  rather than bits, crashes on Kani's output (`map::at`) as of Kani 0.68.

The pattern that follows, and that the payment proofs use: pull a rule out
of the state into a function over plain values (arrays and small structs),
prove that function, and keep the layer that reads the state into it and
writes the result back as small as possible, covered by unit tests.

**For unbounded amounts**, a deductive verifier that hands integer
arithmetic to an SMT solver is the right tool: Creusot (contracts as
attributes on the real Rust, which compile away under `rustc`) or Verus.
Either would prove the payment kernel for every amount. Evaluating one on
that kernel is the next tooling step.

**For whole games**, the plan is a formal model of the rules in Lean 4,
with the engine checked against it by differential testing, the way AWS
built Cedar: a Lean model with proofs about it, and the Rust implementation
tested against the model on millions of generated inputs. See tier 4.

### Running the proofs

    scripts/prove.sh                 # every proof, about six minutes
    scripts/prove.sh <harness>...    # by name

The proofs live in `mtg-engine/src/proofs/` and compile only under
`cfg(kani)`, so `cargo build` and `cargo test` never see them. The
`proofs` workflow runs them on every push to master and every pull
request. Kani is pinned at 0.68.0; `scripts/prove.sh` prints the install
commands when it is missing. Run the proofs one at a time, as the script
does: four at once ran a 15 GB machine out of memory.

## The tiers

### Tier 1: pure kernels against rules-derived specs

Pieces of the engine that are arithmetic or combinatorics over a few
values: where a model checker is complete and fast. Each gets a spec
written from the CR, proved for every input in the space.

| target | spec (CR) | status |
|---|---|---|
| paying a cost from the pool: `can_pay`, `auto_pay`, `auto_pay_reserving` | 107.4, 601.2h | **done**, three defects fixed |
| combat damage division, `combat::divide_combat_damage` (already pure) | 510.1c-d: the amounts sum to the power; nothing reaches a later blocker, or tramples over, before every earlier blocker has lethal; the last blocker of a non-trampler takes the rest | next |
| X funding from the pool, `funding::allocate_for_x` | 107.3, 601.2f, and that floating mana spent on X leaves the rest of the hand payable | next. Its pool step takes all of one type before the next, in the order `generic_payment_order` gives, which is the pattern defect 2 below was; whether that costs a hand its spell is the first thing this proof would settle |
| autotap planner soundness: a plan it returns pays the cost | 601.2g-h, 605.3 | planned; `autotap_brute_force.rs` is the sampled version |
| autotap planner completeness: if some set of sources can pay, it finds one | 601.2g | planned |
| power and toughness from counters and effects (layer 7) | 613.4, 704.5q | planned; needs extracting from `state.rs` |
| lethal damage, deathtouch, and what is destroyed | 704.5g-h, 702.2b | planned |
| the turn's step sequence and who gets priority | 500.1, 117.3 | planned |

### Tier 2: engine invariants as theorems

The fuzz oracle's properties (`check_core`, `check_settled`) are already a
formal statement of what a coherent game state is. Tier 2 proves that each
primitive that changes the state preserves them: moving an object between
zones keeps every card in exactly one zone (CR 400.1) and `library_order`
consistent with the library zone (CR 401.2); tapping, counters, damage and
a pass of state-based actions keep the rest.

The `BTreeMap` measurement above says this cannot be done by pointing Kani
at `GameState`. The ways forward, in order of preference:

1. Extract the bookkeeping each invariant is about (zones and library
   order; the attachment graph; combat's maps) into kernels over fixed-size
   arrays of a few objects, which the state calls. Prove the kernels.
2. Prove the invariants of a Lean model of those primitives (tier 4) and
   test the engine against the model.

### Tier 3: the cards

"Does this card do what its Oracle text says" needs a formal reading of the
Oracle text. The plan:

1. **A card-spec language.** A small, typed language of effects in which
   each card's Oracle text is transcribed: "deals 3 damage to any target",
   "target creature gets +3/+3 until end of turn", "when this enters,
   create a 1/1 white Spirit creature token with flying". The transcription
   is the reviewed artifact, checked against the Oracle text by reading,
   as acceptance tests are now. It is deliberately not the engine's
   `CardBehavior` code: it is an independent statement of what the card
   means.
2. **A reference interpreter** for that language, written from the CR and
   sharing no code with `src/cards/`.
3. **Refinement checks**: for each card, on small boards, the engine and the
   reference interpreter agree. On the engine's real state that is a
   differential test over the states the fuzzer reaches: weaker than a
   proof, but with the same independent oracle. Where a card's effect is a
   kernel in the tier 1 sense, it can be proved.

The order, cheapest first:

- vanilla and keyword-only creatures, whose behaviour is all engine and
  whose data is already checked against Scryfall: the spec is the data;
- the core cards in `src/cards/*.rs` (Lightning Bolt, Giant Growth, Doom
  Blade and the rest), one effect each;
- Innistrad by mechanic, in the groups the `cards_*.rs` files already use.

### Tier 4: a model of the rules

A Lean 4 model of the turn, priority and stack machine (CR 500-514, 117,
405, 608), to prove things about whole games rather than single steps:
that priority passing ends (CR 117.4), that the stack resolves last in,
first out (CR 405.5), that state-based actions reach a fixed point (CR
704.3). The engine would be checked against the model by running both on
the same action traces, which the fuzzer already generates. This is the
largest piece and comes last, once tiers 1 to 3 have shown which parts of
the engine's structure the model should mirror.

## Proved so far

### Mana payment

`mtg-engine/src/proofs/mana_payment.rs`, over the kernel in
`mtg-engine/src/mana.rs`. Every public payment function is one line over
three pieces: `pips_of` (a cost as per-type and generic counts),
`pay_fixed` / `pay_around` (paying those counts out of six per-type
amounts), and `amounts_of` / `write_amounts` (reading the pool and writing
the result back).

The spec: a coloured pip is paid only with its colour, `{C}` only with
colorless, generic with anything, and `{X}` is zero (CR 107.4, 107.3e,
601.2h). So a pool can pay a cost exactly when it has enough of each type
for that type's own pips, and enough in all (`spec_payable`; this is Hall's
condition for the matching).

| proof | property | bounds | time |
|---|---|---|---|
| `each_type_has_its_own_slot` | the six slots are the six types of mana, each once | none | < 1 s |
| `counting_a_cost_counts_every_pip` | `pips_of` counts each type's own pips and the generic, and they sum to the mana value | costs of up to 4 symbols, which is every printed cost | 3 min |
| `paying_succeeds_exactly_when_the_rules_allow_and_spends_the_cost` | `can_pay` and `auto_pay` succeed exactly when `spec_payable` holds, and a success spends exactly the cost, each pip from its own type | a cost of any length asking at most 3 of each type and 3 generic; a pool of at most 3 of each type | 7 s |
| `reserving_succeeds_exactly_when_the_rules_allow_and_spends_the_cost` | the same for `auto_pay_reserving`, around any reserve | as above, and a reserve of the same size | 2 min |
| `reserving_leaves_the_reserve_payable` | when the pool can pay the cost and the reserve together, paying the cost leaves the reserve payable: the promise of #252, #678 and #840 | as above | 40 s |

Not proved: `amounts_of` and `write_amounts`, which go through the pool's
`BTreeMap` (see the tools section). They are a few lines each, and the
unit tests in `mana.rs` cover them, including that a write touches only
the types it spent from. Storing `ManaPool` as six counts instead of a map,
with a serde form that keeps the save format, would remove this layer and
make the whole of payment provable; it is worth doing when the pool is next
touched.

Also not covered: a cost asking more than 3 of a type or more than 3
generic (Army of the Damned's flashback is `{7}{B}{B}{B}`), and the
counting lemma for costs longer than 4 symbols (a reserve built from a
whole hand's pips can be). Both are limits of the checker, and are what an
SMT-based verifier would remove.

**Checked against mutation**: with defect 2 put back, the reserve proof
fails; with `pay_specific` changed to let a type run short, the payment
proof fails. A proof that cannot fail would pass both.

**Defects fixed** (each has a unit test in `mana.rs`):

1. *A failed payment spent mana.* `auto_pay` paid coloured pips one at a time
   and returned the error with the earlier ones gone: `{W}{W}` out of a
   lone `{W}` left the pool empty. Latent: the only mana ability with a
   cost is Shimmering Grotto's `{1}`, and a generic-only shortfall was
   caught before anything was spent. But `activate_mana_source_reserving`
   returns on that error as if nothing had happened, and
   `pay_cost_with_sources` documents that a failed payment "leaves the game
   state untouched".
2. *Reserving took more than the surplus.* `auto_pay_reserving` ordered the
   types by surplus and then took *all* of the first type before the next:
   `{2}` out of a floating `{W}{W}{U}` beside a `{W}` spell in hand spent
   both Whites (tied with Blue at one spare each, White sorts first) and
   left the Blue.
3. *The surplus was counted before the cost paid its own pips.* `{1}{W}` out
   of the same pool beside the same spell: both Whites looked one spare, so
   after the cost's own `{W}` the `{1}` took the White the hand needed.

All three were found while writing the spec down, before any proof ran.
Kani then found a counterexample to `reserving_leaves_the_reserve_payable`
on the unfixed kernel by itself, in 13 seconds (a pool of 3/1/2/3/2/2 of
C/W/U/B/R/G). Defects 2 and 3 are reachable whenever mana is floating
when a payment is made, and the autotap planner judges whether a plan
keeps the hand castable with the same function.

## Conventions

- A proof is named for its property, in words, like a test.
- A spec lives in the proof's file, is written from the CR or the Oracle
  text, and never calls the function under proof.
- Every bound is stated in the proof's module doc and in the table above,
  with the reason for it.
- A new proof is checked against at least one mutation of the code it is
  about, as above: it has to be able to fail.
- A defect a proof finds also gets an ordinary unit test, so `cargo test`
  keeps guarding it on a machine without Kani.
