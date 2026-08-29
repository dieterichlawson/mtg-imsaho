# Calibrating the card audits against ground truth

The audits keep finding bugs, and the instrument doing the finding is an
LLM — prone to both missing real bugs and inventing phantom ones. Every
finding so far has been judged by the same kind of model that produced it,
so "the audit came back clean" has no measured meaning. This protocol turns
it into one, using the two things we actually possess as ground truth: the
git history of real, confirmed, fixed bugs, and clean card implementations
at HEAD.

The seeding mechanics live in `scripts/calibration/seed_bugs.sh`; the
invariant-fuzzing oracle (`scripts/fuzz.sh`, `mtg_engine::invariants`) is
the complementary instrument whose errors are uncorrelated with any LLM's.

## Protocol A — sensitivity (does an audit catch a real bug?)

1. `scripts/calibration/seed_bugs.sh candidates` lists fix commits whose
   non-test change is confined to one card file (~78 as of 2026-08-29).
   Sample 15–25 spanning bug classes: wrong trigger timing, missing "you
   may", restated target filters, snapshot-vs-live reads, wrong zone/cost.
2. `scripts/calibration/seed_bugs.sh seed <commit>...` re-creates each bug
   in its own worktree at HEAD (the fix's `mtg-engine/src` diff reverted,
   compile-checked). `audits/calibration/manifest.tsv` is the answer key.
3. For each specimen, run a **blind** audit of the affected card *inside
   that worktree* with a fresh agent following
   `.claude/skills/check-card-procedure`. Blind means: the auditor never
   sees the manifest, the fix commit, or this document, and (per the
   procedure's own rule) reads no previous audit logs. The operator who
   seeded the bugs is disqualified from auditing them.
4. Score each audit: **hit** (flags the seeded defect, correctly located),
   **partial** (flags something wrong in the right mechanism but
   misdescribes it), **miss** (PASS, or only unrelated findings).

Sensitivity = hits / specimens, reported overall and per bug class. Classes
with low sensitivity are the classes where a clean audit means least — and
where a structural invariant or fuzz oracle is worth building instead.

## Protocol B — false-positive rate (is a flagged issue real?)

Run the same blind audit on 15–25 cards at plain HEAD, sampled from cards
whose last audit was PASS and whose implementation hasn't changed since.
Every ISSUE the auditors raise gets adjudicated by a separate verifier
against the oracle text with both quotes in hand (the audit procedure's own
evidence rule). False-positive rate = phantom issues / audits.

Together A and B give the audit a measured (sensitivity, FP-rate) operating
point. A future "clean pass over all 249 cards" then translates into an
actual upper bound on expected remaining bugs, instead of a feeling.

## Protocol C — how many bugs are left (capture–recapture)

When two *independent* audit passes over the same card set are available
(different prompts or models, no shared logs — the procedure's
no-prior-logs rule is what makes this valid), the Lincoln–Petersen
estimator gives total bugs ≈ (found by A × found by B) / (found by both),
after adjudicating both finding sets. The estimate minus the union found is
the expected number still hiding — the number that has to trend to zero
before "steady state" is a claim rather than a hope.

## Bookkeeping

- Worktrees live under `/tmp/mtg-calibration/` (`CALIB_WORKTREE_BASE`
  overrides); `seed_bugs.sh clean` removes them and the manifest.
- `audits/calibration/` is transient run state (answer key + scoring) and
  is gitignored; only summarized results belong in `reports/`.
- Score sheet convention (`audits/calibration/results.tsv`):
  `specimen  card  verdict(hit|partial|miss)  auditor-notes`.
