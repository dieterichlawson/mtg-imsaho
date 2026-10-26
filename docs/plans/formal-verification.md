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
`mtg-kernel` depends on `vstd`. The three kernels (59 obligations) verify in
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
    scripts/prove.sh exhaustive        # the bounded exhaustive checks, in release
    scripts/kernel-mutants.py [ID...]  # what the proofs and the tests each catch
    scripts/fuzz-engine.sh [SECONDS]   # the whole engine under libFuzzer (tier 2)

The `proofs` workflow runs both provers, and the exhaustive checks, on every
push to master and every pull request; `nightly-fuzz` runs the coverage-guided
target for 50 minutes a night. The Verus release (0.2026.10.04), its Rust toolchain and the
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
  counts and writing the result back, mapping blockers' ids to positions,
  turning X's source groups into counts and the kernel's taps back into
  mana, reading the board into the turn's four facts and into the matrix
  of blocks the rules allow pair by pair, and acting on a creature's fate. Each is a few lines with unit tests; the combat mapping is also
  covered by the exhaustive division test, and the X pool step is asserted
  against the kernel's proved amounts on every call in a debug build.

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
| autotap planner soundness: a plan it returns pays the cost | 601.2g-h, 605.3 | **checked exhaustively** on every board of up to three sources (29.6 million); not yet proved for every board |
| autotap planner completeness: if some set of sources can pay, it finds one | 601.2g | **checked exhaustively** as above, one defect fixed |
| power and toughness from counters and effects (layer 7) | 611.3a, 613.1f, 613.4 | **checked** against the card-semantics reference (pumps, counters, Auras, Equipment, anthems and lords applied continuously, counts) on every cast, death and combat; not yet extracted into a kernel |
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
tap nothing it did not need.

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

### Cards against their Oracle text

`mtg-engine/tests/card_semantics.rs`. A grammar reads each card's Oracle
text, as Scryfall prints it, into effects; a reference semantics written
from the CR, sharing no code with `src/cards/`, says what those effects do
to a board described by its cards' Oracle data, state-based actions
included; and the engine is held to it on a board of fourteen permanents
and four graveyard cards chosen for their differences.

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
- **Werewolves**, both faces: transformed at an upkeep after a turn with
  no spells, and back after a turn of two, alongside the two werewolves
  already on the board.
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
  sorcery is not offered and any other is (CR 602.5d). **Equip** is one of
  them, read as CR 702.6a defines it, onto every creature the caster
  controls.
- **Triggers on other deaths** ("whenever another creature you control
  dies"): once such a creature has entered, every creature on the board
  dies in turn and every two at once, it among them or not. The reference
  fires the trigger for each death the text names, judged by the dying
  creature as it last was, the watcher seeing deaths it dies with (CR
  603.6c, 603.10a), and puts simultaneous triggers on the stack in every
  order the engine may ask for (603.3b).
- **Upkeep triggers**, on creatures and on Curses: from the end step of the
  turn before each player's turn, with every land tapped, into that turn's
  main phase: damage gone, only the active player's permanents untapped,
  the trigger resolved every way in the upkeep its text names and no other,
  and the draw (CR 514.2, 502.3, 503.1a, 504.1).
- **Combat**: every creature whose text is keywords, combat text and
  statics attacks, unblocked, with the defender declining to block, and
  against each of the others as a lone blocker, under a reference model of
  CR 508-510: vigilance, defender, haste and summoning sickness, flying and
  reach, intimidate, menace, first strike and double strike, deathtouch,
  lifelink, trample with every damage split the attacker may choose,
  "can't block", "can't be blocked [by <types>]", protection from creature
  types, "attacks each combat if able", conditional haste, and triggers on
  combat damage to a player. Then the same fights again wherever a
  permanent standing by changes a combatant: a lord, a creature a static
  asks for, or an Equipment on the combatant. About 2,200 and 10,900
  combats agree; `scripts/prove.sh exhaustive` fights every Equipment on
  every creature against every other, 41,083 combats.

157 of the pool's 268 instants, sorceries, creatures, enchantments, lands
and artifacts are read, and all agree. Every template has been shown to
catch a deliberate bug in a card that uses it, and the engine code under
it, from Lightning Bolt dealing 2 to the untap step untapping every
player's permanents (`reports/verification-mutants.md` lists the 48 of
the latest round: 47 caught, and one that needs a change of control the
grammar cannot yet cause), and the failure names the cast and the
difference. Not yet
read: X, choices made by an opponent, randomness, copies, control change,
transforms other than werewolves', triggers on attacking and blocking,
damage replacement, Equipment with abilities of its own, and costs other
than mana, {T} and sacrificing the source. Each is a template, or a board,
to add; `READ` lists the cards read, and a change that stops reading any
of them fails and names it.

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

Each has a regression test that fails without its fix.

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
