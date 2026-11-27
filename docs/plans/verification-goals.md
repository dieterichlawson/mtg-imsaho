# Verification goals (set 2026-11-27)

What the card proofs were, as measured on 2026-11-27, and why that was
not enough:

- 604 Kani harnesses run a card's own code over the real engine. None
  of them goes through the engine's cast or its stack: each calls the
  card's hook directly.
- About 279 of them fix every value, so each is one scenario, and about
  325 leave a little open, usually which player or a yes/no (counted by
  matching `kani::any` in each harness and the helpers it calls, so the
  split is approximate).
- The engine's own proofs stand in for 165 distinct engine functions
  where one function calls another (`has_keyword`, `move_object`,
  `is_creature`, the target checks, the payment's reserve and the rest).
  Whether a stand-in says what the function does is argued in prose, not
  checked.
- 12 engine proofs do not finish.
- Nothing measures whether the card proofs would notice a wrong card.

"A card works" means: cast in a game, it does what its text says and
nothing else, whoever casts it, at whatever it may target, and the
engine refuses what the rules refuse. The goals below are that, each
with a number a script can count. `reports/verification-scoreboard.md`
(goal 0) is where they are counted; a goal is done when its number says
so, not when a commit message does.

## Why the stack was out of reach, and that it no longer is

One measurement on 2026-11-18 had `resolve_top_of_stack` under a real
card out of memory at 12 GB, and the plan recorded the stack as out of
reach "whatever is open". No cause was found then. Found on 2026-11-27,
a step at a time on Divination with nothing open:

| What runs | Result |
|---|---|
| the card's hook alone | proves, 75 s |
| the hook, and the spell moved where a resolved spell goes | proves, 81 s |
| `resolve_spell_as`: the engine's resolution of a spell, short of reading it off the stack | proves, 83 s |
| `resolve_top_of_stack` | out of memory at 6 GB |
| the stack's steps written out in the harness, its other kinds of entry answered with a panic | still in symbolic execution at 400 s |

So the cost was neither the card nor the resolution, and not the other
kinds of stack entry: it was the spell's id as read back off the stack.
`scripts/precise-enums.py`, which rewrites the program so that CBMC
knows the value of an enum built from known parts, left one case alone
on purpose: a narrow variant of a union wider than 1,024 bits. A stack
entry is 1,664 bits and `StackEntry::Spell(id)` is 64 of them. The
entry was therefore unknown to symbolic execution, the id read off it
was any object's, and everything the resolution did with it was done to
every object. The limit is there for narrow variants that hold a
pointer; a variant with no pointer in it is plain bits. With the rule
extended to those (`--wide-pointer-free`), `resolve_top_of_stack` on the
same board proves in 83 s.

### What each step of the game loop costs

One pass of the game loop (`engine::run_game_loop_inner`) is: what is
checked before a player receives priority (`before_priority`: the
state-based actions and the triggers collected, CR 117.5); the actions
the engine offers (`legal_actions`); the action, by `submit_action`; and
priority handed on (`hand_on_priority`), which resolves the top of the
stack when every player has passed. Measured on this container (4 cores,
15 GB), 2026-11-27, with the driver reading CBMC's plain output; steps
are CBMC's of symbolic execution (one proof of 791 thousand steps
peaked at 3.6 GB).

Divination, its caster's library two cards, nothing on the battlefield:

| What runs | Result |
|---|---|
| the cast, each player's pass and the resolution, by `submit_action_on` and `hand_on_priority` | proves, 95 s, 812 thousand steps |
| `legal_actions` alone | out of memory at 11 GB: 2.39 million steps, then the formula |
| `submit_action`, with its copy of the game | out of memory at 6 GB |
| the same spell from a library of one, which ends the game | over 6 GB: the engine keeps a copy of the final state (`end_game`) |

Doom Blade at the other player's 2/2, a second 2/2 beside it:

| What runs | Result |
|---|---|
| the board alone | 39 s, 287 thousand steps |
| the cast alone (targets checked, cost paid, on the stack) | proves, 88 s, 731 thousand steps |
| the cast, each pass and the resolution (target checked again, destroyed) | proves, 138 s, 1.03 million steps |
| what is checked before priority, alone, on the board as built | proves, 57 s, 500 thousand steps |

So the engine's cast, its stack and its resolution are in reach for a
real card at a real target. The check before priority cannot be left
out and still be the game: `submit_action` clears the event list as
each action begins, so an event not collected before the next action is
a trigger that never happens. Every pass of the loop whole, then:

| What runs | Result |
|---|---|
| Divination, its caster's library three cards and the other player's one: every pass whole | proves: 616 s over three rounds of learning loop bounds, the last 196 s and 1.62 million steps |
| Doom Blade at the 2/2: every pass whole | not finished at 1,500 s, still in symbolic execution |

Two things were learned getting there. First, a proof's own board
decides whether the events are readable at all. With room reserved in
the event list for 40 events, an event read back from it is of a kind
symbolic execution does not know: read off what CBMC was unwinding,
Doom Blade's proof was dropping the list as every kind of event and
collecting triggers through the collectors for a milled creature card,
a death, combat damage and blockers, none of which had happened, and it
ran out of memory at 11 GB (with room for 96 too). With room for 32 the
kind is known, and it is in no such collector. (Probed with one event
in an empty list: known with room for 8, 10, 16, 21, 22 and 32; not
with 40, 42 or 43. Divination ran out of memory at 11 GB with room for
48 and proves with room for 32.) Second, what symbolic execution knew of an event stopped at its kind.
Probed with one event in the list: it knew the object of `Tapped`,
`CardDrawn` and `ManaPoolEmptied`, and not the object of `ObjectMoved`
or `LeftBattlefield`, pushed by a proof or by the engine's
`move_object`. A Kani build lays an enum out as declared, and CBMC
keeps an event as its widest variant, `CreatureDied`; the two that were
lost end in single bytes (a zone, a controller) after their object, and
those bytes fell on the pointer of `CreatureDied`'s `name`. So after a
permanent left the battlefield the trigger collectors looked up any
object for the one that left and any card for its card, and went on to
describe a trigger for it: when stopped, the proof was in the string
search of that description. `CreatureDied` now has, in a Kani build
only, its plain fields before its string and lists, and the damage
events their amount first, so that no narrower event reaches a pointer
of it. With that:

| What runs, a creature beside the permanent | Before | After |
|---|---|---|
| a creature returned to its owner's hand by `move_object`, then what is checked before priority | not finished at 360 s | proves, 92 s |
| an artifact destroyed by `try_destroy`, then the same | not measured | proves, 101 s |
| a creature destroyed by `try_destroy`, then the same | not finished at 360 s | not finished at 420 s |

The last row is what is left: `CreatureDied` itself, read back from the
list, is of no known object or controller (the same probe). Why is not
found yet. A guess, not measured: the event is built from the dying
creature's toughness and subtypes as the layers read them, CBMC carries
a value forward only when it knows all of it, and one of those is not a
value it knows.

28 existing proofs that build or collect these events were run again
with the new order and all prove: 7 of `proofs::actions::damage`, 6 of
`destruction`, 5 of `trigger_resolving`, 1 each of `triggering` and
`moving`, and 8 of the card proofs. (The commit that made the change
says nine card proofs; it was eight.) The other harnesses have not been
run again with it.

So on this machine: a spell after which no creature dies goes through
every pass of the loop, and so does a cast the engine refuses. A spell
that kills a creature goes through the cast, the stack and the
resolution, and not yet through the check after.

Two more limits of this machine: Kani's own compiler is stopped for
memory building four of these proofs at once (two at a time build), and
each proof is then 80 to 110 MB of program that CBMC's tools each read
and write before CBMC starts.

### What a run of them takes, and where it is to run

The first batch on this container (4 cores, 15 GB), one proof at a time,
gave 10 results in 107 minutes, of which 58 were the solver's and the
rest the builds and the tools before it: Bramblecrush destroying an
artifact of each player's, an enchantment and a land (315 to 359 s
each), Doom Blade at a creature with a regeneration shield (439 s) and
refused at a black creature of each player's and at an artifact (62 to
71 s) prove; Doom Blade at an indestructible creature was still in
symbolic execution at 1,200 s, and why is not found. At that rate the
267 proofs written and not yet run are two days of this container, and
a background job here is stopped at two hours.

So the run is made one that can go elsewhere, in
`.github/workflows/proofs-full.yml`, dispatched by hand:

- `harnesses: new` runs only the proofs with no result yet
  (`scripts/prove.sh kani-new`), and is red only if one is left with
  none: a new proof that does not finish is what it is to find out.
- `machine: github` (the default) shares them over 30 of GitHub's
  runners, which are this container's size: the volume, not the memory.
- `machine: aws` rents one machine for the run, proves every share on
  it at once (`scripts/prove.sh kani-shares`) and terminates it
  (`scripts/ec2-run.sh`): 32 cores and 256 GB unless told otherwise,
  and a solver may be given more than 12 GB there.

Neither has been dispatched yet (2026-11-27): the first waits on a word,
the second on the repository's AWS secrets. `scripts/ec2-run.sh` has
been run against stand-ins for AWS and SSH only.

A proof's result now also says which of its own assertions no execution
reached. A proof here is one case of a family that asserts under each
case's branch, so some are expected: Bramblecrush destroying a land
reaches the assertions of a permanent destroyed and not those of one
left as it was or of a shield used, and Doom Blade at a regenerating
creature the reverse. An unreached assertion that is the case's own
would be a proof of nothing, and is now visible as one.

## The goals

**0. A scoreboard.** `scripts/verification-scoreboard.py` writes
`reports/verification-scoreboard.md` from the sources and the recorded
results: for each of the 270 cards, whether it has a proof through the
stack, how many cases, and its mutants killed; and the counts for goals
3 to 6. Done when it runs in CI and fails on a number that went the
wrong way.

**1. Every card is proved through the game, not through its hook.** A
spell: in its owner's hand, cast by the engine (targets checked, cost
paid), each player passes, resolved off the stack by the engine, the
state-based actions and triggers checked, each step as the game loop
plays it (`proofs::through_the_stack`). A permanent: cast the same way,
and its triggered and activated abilities put on the stack and resolved
by the engine, not called. Baseline 0 of 270. Done at 270, or with each
card that is not listed beside the measurement that says why. The 604
hook harnesses go as each card's replacement proves.

Two parts are not there yet, each for a measured reason above, and each
is this goal's to close: the action is named by the proof and not taken
from `legal_actions` (which alone is past 11 GB), and a spell after
which a creature dies does not finish the check before priority.

**2. No card's proof is one scenario.** Each card's proofs through the
stack cover at least: each player casting; each kind of thing its text
lets it target, under each player; one thing its text does not let it
target, refused at the cast (CR 601.2c); for a spell with a target, the
target gone or made illegal before it resolves (608.2b); every amount
its text reads (X, "for each", a count) at none, one and more; and a
second permanent on each side that must come out untouched. A value
the solver cannot hold open is a harness for each value. Baseline: no
card. Done when the scoreboard shows every card with every row that
applies to it.

**3. The proofs are shown to catch wrong cards.** For each card, its
code is changed one way at a time (a number, a player, a zone, a filter,
a line removed), and the card's proofs and small-board checks must
fail. Baseline: not measured. Done when every such change to every card
is caught, or is shown to change nothing a game can see.

**4. No engine proof rests on an unchecked stand-in.** For each of the
165 engine functions some proof stands in for, either the proof runs
the function itself (the fix above may let many), or the stand-in has a
proof that it answers as the function does for what the proof asks.
Baseline 165 unchecked. Done at 0.

**5. Intent from someone else.** The Oracle cache holds 578 official
rulings on 175 of these cards. Each is either played through the engine
and holds, or is marked as not a statement about behaviour. Baseline:
not tracked. Done when all 578 are accounted for.

**6. Nothing left open.** The 12 engine proofs that do not finish, One-
Eyed Scarecrow (the one card with no engine proof), and `submit_action`
with its copy. First tried again with the fix above; what still does
not fit is a question of memory, and is what a larger machine is for.

**7. Every card on the small boards.** 129 of the pool's permanents can
be pieces of the exhaustive small boards; the other 70 (Auras,
Equipment, double-faced cards, upkeep and end-step triggers and the
rest) are only ever the card being played. Done when all are pieces,
and when the boards of three pieces have been played whole for every
check, which is hundreds of core-hours.

## What this will not be

A proof that the engine implements the rules for every game of any
length on any board. That is a different kind of work (a specification
of the whole game and a deductive proof of 50,000 lines against it),
measured in person-years. The kernels proved without bounds in Verus
(mana payment, power and toughness, the state-based predicates, turn
structure, blocks) are the part of it that exists. What the goals above
give is every card through the real game on bounded boards by proof,
every card on every small board by exhaustion, and evidence, by
mutation and by the rulings, that those would notice a wrong card.
