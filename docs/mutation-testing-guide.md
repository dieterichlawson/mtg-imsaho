# Which surviving mutants are worth fixing

`cargo-mutants` makes ~2,400 small deliberate edits to the engine and reports
the ones no test noticed. This is a guide to deciding what to do about them.
It exists because the obvious answer — kill them all — is the wrong one, and
following it for a while made parts of this suite worse.

## The instrument is not a score

Mutation testing answers one question: *is there a place the code could be
wrong where no test would say so?* It is a finder, in the same family as the
fuzzer and the playtest crew. A survivor is a lead, not a defect.

What the field says about treating it as a score to maximize is consistent:

- ~80–85% is a strong result for a production codebase, and the last stretch
  to 100% is where you start "writing fragile tests that exist only to kill
  specific mutants, not to validate behavior."
- Google, running this at their scale, does not show developers most of the
  mutants it generates. They separate **productive** mutants from **arid**
  ones — unreachable, semantically equivalent, or trivially uninteresting —
  and suppress the arid ones automatically, because surfacing them wastes
  reviewer attention and trains people to ignore the tool.
- `cargo-mutants`' own stated goal is that "most, ideally all, findings should
  indicate something that really should be tested more", and it ships
  `#[mutants::skip]` precisely because some mutants are not worth testing.
- Equivalence is undecidable in general, so a residue of unkillable survivors
  is not a defect in the suite. It is arithmetic.

The cost of getting this wrong is not wasted time; it is a suite that pins the
current implementation in place. A test written to kill a mutant in the middle
of a computation asserts *how* the answer was reached. The next person to
improve that computation has to rewrite the test, and cannot tell from reading
it whether they broke a rule or merely picked a different route to the same
answer. That is how a test suite stops protecting a codebase and starts
holding it still.

## The two questions

Before writing a test for a survivor, in order:

**1. If someone made this exact edit and shipped it, what would go wrong that
a person could see?**

Name the symptom concretely: a player is offered an illegal play, a spell
resolves against the wrong target, a game hangs, the fuzzer stops reporting a
class of violation. If you cannot name it, the mutant is arid. Accept it and
write down why.

**2. Would my test still pass if this function were rewritten correctly a
different way?**

If a legitimate refactor breaks the test, the test is pinning the
implementation. Do not write it. Go and find the property that the
implementation serves, and test that instead (see *One property beats N
constants*, below). If there is no such property, that is a strong sign the
mutant is arid.

A survivor that fails question 1 goes to the accepted list. A survivor that
passes 1 and fails 2 usually means the *class* needs one property test, not
that this mutant needs one pinned assertion.

## Fix these

1. **A rule.** The mutant changes what the Comprehensive Rules say happens —
   a target that should not be legal becomes legal, a trigger fires on the
   wrong event, damage lands on the wrong object. Cite the CR rule in the
   test.
2. **A contract.** The mutant makes a function break a promise its callers
   depend on. `compute_autotap` promising a plan that pays the cost; a
   requirement enumerator promising the candidates it names are legal.
3. **An oracle clause.** The mutant blinds the invariant checker, a
   state-based action, or the fuzzer's own reporting. These are the highest
   value per test in this repo: the checker is the only pair of eyes on
   ~110k games a night, so a blinded clause means that whole class of bug
   goes unseen *and the run stays green*. Nothing else in the suite covers
   this, because the checker is normally the thing doing the covering.
4. **A boundary a card actually reaches.** `<` vs `<=` on loyalty, on hand
   size, on the number of blockers — where the pool contains a card that
   sits exactly on the boundary.
5. **A silent failure path.** Code where "there is nothing to do" and "I do
   not know how to do this" return the same value — an empty candidate list,
   a `None`, a catch-all match arm. These are worth a test even when no
   current card reaches them, because the failure mode is invisible: the
   spell is simply never offered. `generate_ability_targets` had eight
   requirements falling into a catch-all that meant "no legal target", and
   the only reason it never fired was that no card in the pool used one.

## Accept these

Each goes on `reports/mutants-accepted.txt` with a reason recorded here or in
`reports/mutation-testing.md`. The reason is the deliverable — an accepted
mutant with no reason is indistinguishable from an ignored one.

1. **Equivalent.** The mutated program cannot behave differently. `> 0` vs
   `>= 0` on a value an early return already proved non-zero.
2. **Unreachable with the current pool.** No card produces the input. Note
   that this is *accept*, not *delete* — unless the code is also a duplicate
   of something else, in which case delete it; see below.
3. **Heuristic or preference.** The code is choosing among answers that are
   all correct. Which land to tap, which order to enumerate equal options,
   which of several legal plans to offer. Pinning these freezes a knob you
   expect to turn.
4. **Internal bookkeeping of a result that is already tested.** The
   accumulator inside a loop whose output has a property test.
5. **Display, log wording, and ordering nobody depends on.** A log line is
   worth testing when a person reads it to understand a game (see
   `log_attribution.rs`); its exact phrasing is not.
6. **Performance-only.** A cache, an early exit, a capacity hint. Determinism
   and results are the contract; speed is not asserted.
7. **Already covered by a property.** If a property test would catch any
   *behavioural* break of this line, a second pinned assertion adds nothing
   but rigidity.

## Delete these

Some survivors are telling you the code should not exist:

- A second implementation of something the codebase already does once. Two
  copies drift, and the copy no test reaches is the one that drifts silently.
- A match arm, branch, or helper that nothing can reach and that duplicates a
  reachable one.

Deleting is better than accepting *and* better than testing: it removes the
mutant, the drift risk, and the reader's confusion in one move. When you
delete, say in the commit message which survivors went with it.

## One property beats N pinned constants

The worked example, from this repo:

`compute_autotap` decides which mana sources to tap. Twenty-three mutants
survived inside its internal simulation — every `-=`, every `+=`, every
`> 0`. Killing them one at a time would have taken twenty-three assertions
about intermediate arithmetic, and would have written the current heuristic
into the suite. The planner has already been retuned twice (issues #114,
#252) and will be again.

The contract underneath is two sentences, and does not move:

- a plan the planner offers is one the payment can actually execute
- a `None` means no plan would have worked

Two property cases — run every offered plan and check it pays; check every
declined case against an exhaustive search of all plans — catch the same class
of bug, catch bugs the twenty-three assertions would have missed, and leave
the heuristic free to change. That is the trade to look for.

When a cluster of survivors sits inside one computation, the right response is
almost always one property over its output, not one assertion per line.

## Ceremony worth keeping

- **A mutation-motivated test is not done until it has been watched failing
  under its exact mutant.** Four of an early round's claimed kills were false
  — tests that passed vacuously under the mutation they were written for.
  Apply the mutant by hand, run the test, see it fail, restore.
- **Write the test for the behaviour, then check it kills the mutant** — not
  the other way round. A test derived from the mutant tends to describe the
  mutant.
- **Name the symptom in the test name.** `an_up_to_n_second_slot_may_be_left_empty`
  survives a refactor; `kills_lt_le_mutant_at_line_412` does not.

## Where a survivor ends up

| Bucket | File | Ceremony |
|---|---|---|
| Killed | — | Delete its line from the backlog; the test is the record |
| Accepted | `reports/mutants-accepted.txt` | One line, plus a written reason here or in `reports/mutation-testing.md` |
| Deleted | — | Say so in the commit message |
| Backlog | `reports/mutants-backlog.txt` | Genuine gap, not yet worked; the weekly workflow will not re-file it |

The weekly workflow files an issue only for survivors in none of these
buckets, so an untriaged survivor is the only kind that becomes an issue.

## Budget

There is no target score. A file whose every survivor falls in an *accept*
bucket is finished at whatever percentage that leaves. Working the backlog is
worth doing when the leads are productive and worth stopping when they are
not; a session that closes ten arid survivors with written reasons has done
more for the codebase than one that adds ten rigid tests.
