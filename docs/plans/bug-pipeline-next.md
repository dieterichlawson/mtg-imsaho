# The bug pipeline after the first five weeks

Written 2026-10-05, from a read of `reports/playtests/` (37 nights,
2026-08-29 to 2026-10-05), the tracker (657 issues, 9 open at the time of
writing) and the three routines that feed it.

## Where it stands

The severity of what the nights find has fallen, and the reports say so
themselves: "the new defects are one level further out", "all three
defects are one layer out from the rules", "both defects are presentation".
The early nights found rules bugs — an attacker that left the battlefield
still blockable, a copy effect wired as a trigger, trample past a
planeswalker spilling to its controller. The last two weeks of nights
found, in order of how often: a fix that reached one surface and not the
others; the mana planner stranding a spell; the LLM harness silently
changing a schema-legal answer; a label, a log line or a summary
describing a correct result wrongly; a process-lifecycle wedge. The
"checked, correct" sections dominate every recent report.

Night totals fell further than that suggests only because the crew shrank
from fifteen probes to three or four. Per probe the yield is flat, at
two to four issues, but each find now costs a code read plus a brute
force rather than a game. The tracker keeps up: nearly everything closes
the day after it is filed, and the open set is design questions.

| Night | Probes | Issues | Sharpest find |
|---|---|---|---|
| 09-01 | 15 | 28 | Evil Twin copies via an ETB trigger; a sacrificed attacker still blockable |
| 09-02 | 15 | ~42 | A duplicate attacker index multiplies a trigger and wins a game |
| 09-05 | 16 | ~39 | Trample past a planeswalker spills to its controller |
| 09-24 | 3 | 1 | LLM menu key coarser than the engine's |
| 09-29 | 2 | 3 | CLI keys a cast by whether an alt cost exists, not which |
| 10-01 | 4 | 15 | A sac-cost ability hidden unless mana floats; a response window skipped |
| 10-04 | 3 | 12 | LLM seat misreads two schema-legal answers; random seat never sends ChosenOrder |
| 10-05 | 3 | 8 | Auto-tap strands the next spell; a Lands line drops an entry |

Two things drive the decline. The Innistrad pool is close to exhausted as
a source of rules edges: the reports record "no card in the pool reaches
this" and file the gap as a latent idea. And the fuzzer covered less than
anyone assumed: the random seat went weeks without sending an ordering
answer and reached one card's second mode in under two percent of casts,
with the nightly job green throughout.

## What changed on 2026-10-05

Seven changes, in the order they were made; the decisions on the nine
open tickets are recorded on the tickets themselves.

1. **The auto-tap brute force is a test.** `mtg-engine/tests/
   autotap_brute_force.rs` compares `compute_autotap` with a hand-tap
   oracle over seeded boards: a plan exists whenever a hand-tap pays, the
   plan executes, it strands no hand spell another plan would keep, and it
   taps nothing it did not need. The 10-05 crew did this in a throwaway
   crate and found three issues; it now runs on every change. Cost shapes
   no card has yet (three colours, `{C}` pips, repeated pips) were tried
   once by hand and the result is in the test's doc comment.
2. **Fix propagation is a step in the fixer's routine.** A fix that
   touches a prompt, a row, a label, a schema, a log line or a game loop is
   checked on the other three surfaces and in the draft runner's copy
   before the issue closes, and the closing comment says which. The crew
   manual's new standing probe, R1, audits it each night.
3. **A cross-surface parity test.** The CLI and the LLM seat key their
   menus through one shared function in `mtg-player/src/lib.rs`, and
   `mtg-player/tests/surface_parity.rs` (or its in-crate equivalent)
   plays seeded games and asserts, at every priority menu, that the two
   surfaces show the same offers, that no two rows are byte-identical,
   and that every row maps back to exactly one engine offer. This is the
   check the 09-29 report said nothing performed.
4. **The fuzzer reports its reach.** `mtg-runner --decision-stats`
   counts the kinds of question each game asked and the widest board it
   built; `scripts/fuzz_reach.py` sums a campaign and names every kind
   the engine defines that no game reached; the nightly workflow
   publishes the table and files the gaps as `phase:fuzz` issues. Two
   deck sets were added beside the singleton coverage decks:
   `decks/fuzz/*-doubles.txt` (every spell twice, so two same-named
   objects can share a board) and `decks/fuzz/flood/` (a token engine
   played as a mirror, a few seeds a night, under the per-game timeout).
5. **Severity labels.** `sev:rules`, `sev:game-affecting`,
   `sev:presentation`, defined in the crew manual's Filing section. The
   fixer works severe-first. The nine open issues carry them.
6. **The card pool.** `docs/plans/dark-ascension.md` plans the next set.
   It is the only lever that brings back the rules bug class.
7. **The nightly crew's order of work.** Propagation sweep first, then
   untried ideas, then the two-week floor. Rules-lawyer re-probes on the
   floor alone were producing "every CR claim held" nights.

## What to watch next

- Whether the nightly fuzz files reach gaps and the random seat gets
  fixed to close them, or whether the gaps are deck-set gaps.
- Whether the doubles and flood campaigns find anything in their first
  week. Under random play the flood mirror reached 18-90 permanents on
  twelve seeds against the coverage decks' 25-31: wider, not wide. The
  reach report prints the widest board each night; if it never passes a
  few hundred, the campaign needs a staged save rather than a deck.
- Whether R1 keeps finding escaped fixes. If it goes quiet for two
  weeks, the fixer's propagation step is doing its job and R1 can drop to
  weekly.
- The planner test's "unmet shapes" result. If it found misses, those
  become issues the day a card with such a cost is implemented, and the
  Dark Ascension plan names which cards those are.

## What changed on 2026-10-08

Three things landed in one day, and each one is a surface the loops did
not know about until now.

1. **The LLM seat is stateless, and the bill is measured.** One decision
   is the system prompt plus one prompt (`MTG_LLM_HISTORY=0`), the
   thinking level is a knob (`MTG_LLM_THINKING`), and
   `reports/llm-cost.md` is the before/after table with the method to
   re-measure (`scripts/measure-llm-prompts.sh` under a stub `claude`,
   free; a real `cc` game for the token counts). The request-shape tests
   pin the bound. `harness.md` H26-H28 are the probes: the bound holds,
   what the seat lost without the history, and what it is no longer
   asked. CLAUDE.md says a prompt change is measured before it lands.
2. **The hosted table.** `mtg-draft-server` seats people (browser page or
   `mtg-draft-client`) and AI seats at one draft, builds, and plays the
   matches through the game pages. Its view, its page and its client are
   three more copies of every draft decision, so the fixer's propagation
   step and the crew's R1 sweep name them. `drafting.md` D27-D32 and
   `gui.md` G27-G28 are its first probes; `reports/playtests/2026-10-08-
   draft-with-friends.md` is its first night, played before it shipped.
3. **A workspace test workflow.** Until today nothing ran `cargo test`
   on a push; the fixer ran it locally and the nightly instruments ran
   only their own binaries. `.github/workflows/tests.yml` now checks,
   clippies and tests the workspace on every push to master, so a fix
   that lands green locally and red on a clean checkout is seen the same
   hour, not the next night.

What to watch: whether the per-game token figures in the cost report
drift up as prompts grow (H26 reads them; the fuzz workflow's reach table
is the model for publishing them), and whether the hosted table's first
real nights (a person at it, not Playwright) find what the stub could
not.
