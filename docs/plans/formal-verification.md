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
  a `KindMap` instead, each proved an ordered map (below).
- **CBMC's SMT backend** (`--z3`) crashes on Kani's output (`map::at`) as
  of Kani 0.68.

The pattern every proof so far follows: pull a rule out of the state into a
function over plain values (arrays, slices, small structs), prove that
function in `mtg-kernel`, and keep the layer that reads the state into it
and writes the result back small, with unit tests on it.

### Running the proofs

    scripts/prove.sh                   # Verus, then Kani: about four minutes
    scripts/prove.sh verus             # mtg-kernel only: seconds once vstd is built
    scripts/prove.sh kani [NAME...]    # the Kani proofs, or the named ones or modules
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
included; and the engine is held to it on a board of fifteen permanents
and six graveyard cards chosen for their differences, and on variants of
it: after a death, with a second permanent of a name, with one of the
opponent's creatures tapped.

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
  transformed since the ability went on the stack (701.27f); and every
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
  text does not print.
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
  502.3, 503.1a, 504.1).
- **End-step triggers**, either face up, with intervening "if"s (morbid,
  "didn't attack this turn"): from each player's postcombat main phase,
  on the board and after a death, through the end step and cleanup into
  the next turn's main phase, with the face up by then (513.1a, 603.4).
  Both turn tests also count what the engine puts on the stack against
  the triggers the text fires, so one that triggers when its "if" is false
  and then does nothing is seen.
- **"You may"**: besides every ending reached, the death checks play
  every "may" taken and every one declined, since as a set of endings a
  wrong answer to one can hide behind a right answer to another. A "may"
  with a cost ("you may pay {1}. If you do, draw a card") is taken only
  when the mana floating pays it, and played with none left.
- **Triggers on spells cast and creatures entering**: with the watcher on
  the board, its controller casts an instant from hand and by flashback
  and a creature, and the opponent casts in the game loop, so that what a
  cast sets off is collected; a creature entering is judged as it is on
  the battlefield (CR 601.2i, 603.6a).
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

224 of the pool's 268 instants, sorceries, creatures, enchantments, lands
and artifacts are read, and all agree. Every template has been shown to
catch a deliberate bug in a card that uses it, and the engine code under
it, from Lightning Bolt dealing 2 to the untap step untapping every
player's permanents (`reports/verification-mutants.md` lists the rounds:
of the latest 190 mutants, 187 caught, one missed for want of a change of
control between activation and resolution, two equivalent), and the
failure names the cast and the difference. Every mutant that first
survived became a check or a board: the "may" policies, the stack count,
"nothing beyond its text", the opponent's Vampire, the declarations
test, the opponent casting in the game loop, a Bears cast with no mana
left. Not yet read: copies, replacement effects ("would deal combat
damage, deals double", token doubling), counters other than +1/+1 and
-1/-1, rule modifiers (cost reductions, "can't be activated", "can't be
cast"), modal spells, protection and hexproof granted, attack triggers
with targets, links between abilities ("the exiled card"), and costs of
tapping other creatures or removing counters. Each is a template, or a
board, to add; `READ` lists the cards read, and a change that stops
reading any of them fails and names it.

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
| `attach` (701.3a, 701.3b, 702.6c) | proved, 82 s |
| `add_counters`, `add_counters_quiet` (122.1) | proved, 86 s |
| `remove_counters` | proved, 74 s |
| `add_regeneration_shield` (701.15a) | proved, 51 s |
| `set_base_pt` (613.4b) | proved, 22 s |
| `gain_control_while_source_controlled` (613.1b) | proved, 97 s |
| `player_loses`, `end_game` (104.3) | proved, 37 s and 52 s |
| `draw_cards` (121.1, 121.2, 704.5b, 614.1), a library of 0 to 3 | proved, 120 to 231 s |
| `mill_cards`, `mill_one` (701.13a), a library of 0 to 2 | proved, 54 to 190 s |
| `shuffle_library` (701.20a), `choose_at_random` of 0 to 4, `flip_coin`, `random_below` | proved, 6 to 67 s |
| `counter_spell`, `counter_spell_exiling` (701.5a, 702.34a) | proved, 319 s |
| `apply_pending_effect`: damage, destroy, sacrifice, return to hand, -N/-N, can't block, a card's own effect, a search's answer | proved, 64 to 173 s each |
| the targeting predicates (`is_creature`, colours, card types, keywords, filters, `can_be_targeted_by`) | proved, 13 to 103 s each |
| an Aura's or Equipment's printed effect reaching the creature it is attached to (613.1, 611.3a): each P/T change, keyword and Human condition the pool prints, can't attack, can't block, attacks each combat, doesn't untap | proved, 36 to 76 s each |
| `try_destroy`, `try_destroy_by`, `try_destroy_no_regen`, `sacrifice_by` (701.7a, 702.12b, 701.15a, 701.17a), each its own harness | proved, 154 to 174 s each |
| `present_target_choice` (a choice of target applied at once or asked) | proved, 149 s |
| `fight` (701.12a) | proved, 113 s |
| damage on a board printing no effect: only protection applies (615) | proved, 62 s |
| tokens entering attacking (506.3, 508.4), a token on a board without a planeswalker | proved, 161 s, at seven gigabytes |
| `prune_effects_on_departed_objects` (400.7): each kind of effect about either object or one gone, and two or three in order | proved, 52 to 198 s each, but the prevention's (past four gigabytes) |
| `damage::perform` (120.3, 702.2b, 702.15b) on either player | proved, 212 and 219 s |
| damage nothing prevents or replaces, through the replacement layer (616.1), to either player | proved, 89 and 91 s |
| `finish_library_search` (701.19a, 614.1c, 302.6) | proved, 133 s, once defect 12 was fixed |
| `arrives_tapped` (614.1c), a change of control, adding mana to a pool | proved, 49 to 65 s each |
| the creature groups a card's effect names ("creatures you control", "your opponents control", "creature tokens you control", "with flying your opponents control", "as long as you control a Human", "as long as an opponent controls a Human"), for a card and a token | proved, 32 to 65 s each |
| `replacement::for_entering` on a board printing no replacement (614.1) | proved, 39 s |

Three of these held only once the function asked what the rule says:
`untap` untapped a card off the battlefield, `discard_card` discarded a
permanent, and `attach` left an Aura attached to the player it came off;
no game reached any of them, every caller offering only what the rule
allows, and each has a regular test besides.

Not yet finished: moving a card between zones (each past two and a half
gigabytes, to be run at a larger cap); destroying permanents together
(past eleven); the damage queue, `perform` on a permanent, and
prevention and doubling, a harness for each target, board and set of
effects (those on a permanent, and those a doubler or a shield applies
to, past four); the other boards of tokens entering attacking (past
four), putting a card into a library, the prevention's pruning, paying
a cost (the autotap planner's symbolic execution past half an hour, a
pool the solver chose or not), the subtype filters and the creature
groups that name a subtype, a filtered target requirement, a search's
question and discarding several cards (past two and a half), and the
`KindMap` proofs (heavy, to be run alone at a larger cap). Of the cards,
Full Moon's Rise's ability, Vampiric Fury, Moonmist, Spare from Evil,
Rolling Temblor, Make a Wish, Mulch, Balefire Dragon, Hamlet Captain,
Tribute to Hunger, Sever the Bloodline, Blasphemous Act, Past in Flames,
Ghoulcaller's Chant, Nevermore, and the ability lists of Grimoire of the
Dead, Back from the Brink and Moorland Haunt and Ghost Quarter's
resolution on a permanent, are past four gigabytes even a case to a
harness, to be run alone at about ten. The results so far were of
several commits. A run at one commit, 481c7ce, reached 722 harnesses and
was stopped: the payment, effect and zone changes above reach code under
nearly every proof. A run at e63d2cf takes the harnesses not yet run;
then every harness is run again at one commit, those known to need more
memory alone, and this section will record which hold there.

A counterexample can be of a board no game reaches, which is then the
board's error and not the card's: the board the counted creatures are
proved over gave each card a power, a creature or not, and the engine
takes an object with a power for a creature, so Boneyard Wurm counted a
non-creature card. A noncreature has no power (CR 208.3); the board now
gives one only to a creature, and the proof holds. And a failure can be
the proof's own: Memory's Journey's built its two targets in a list with
room for one, and failed on the list growing, whatever the card did; and
holding the ids of new tokens to eight failed Army of the Damned's
thirteen, until they had room of their own.

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
| 7 | an ability transformed its own source again after the source had transformed since it went on the stack (CR 701.27f): Moonmist in response to a werewolf's upkeep trigger was undone by the trigger, a second Bloodline Keeper activation held under the first turned Lord of Lineage back, and held Ludevic's Test Subject activations flipped the Abomination back. A test asserted the first | writing the reference's "transform this creature" for Thraben Sentry, from 701.27f | any response that transforms a permanent whose own transform ability is on the stack |
| 8 | Daybreak Ranger chose between its faces' abilities, both index 0, by the face up at resolution, so Moonmist in response to "deal 2 damage to target creature with flying" made Nightfall Predator fight it; Homicidal Brute's end-step ability did nothing at all once turned back in response | the check of every double-faced ability with its source transformed in response | Moonmist, or any transform, in response to either |
| 9 | Unbreathing Horde entered with a counter for each Zombie of the player who last controlled it, not of the player it was entering under: returned by an opponent's Grimoire of the Dead, it counted its owner's Zombies | writing its Kani card proof, whose replacement is offered the Horde entering under either player | any effect that puts the Horde onto the battlefield under a player other than its last controller |
| 10 | Garruk, the Veil-Cursed's −1 sacrificed the activator's creature and then searched the library of whoever controlled Garruk as it resolved, not the activator's (CR 602.2a) | writing its Kani card proof, which separates the activator from the controller | a Garruk whose control changes with the ability on the stack |
| 11 | an ability an Aura grants was offered only to a player controlling the Aura, so a Skeletal Grimace on an opponent's creature gave its regeneration to no one (CR 602.2: the enchanted creature's controller activates it) | reviewing Skeletal Grimace for its Kani card proof | any Skeletal Grimace on a creature its controller does not control |
| 12 | finishing a library search cleared the summoning sickness of whatever it put onto the battlefield, so a creature card a search put there could have attacked, or paid a {T} cost, the turn it arrived (CR 302.6) | the Kani proof of `finish_library_search`, which says nothing is done to the card but what entering does | latent: the pool's one search onto the battlefield is Ghost Quarter's, for a basic land |

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
