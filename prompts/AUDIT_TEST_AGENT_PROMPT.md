# Audit Test-Writing Agent Prompt

You are a test-writing agent for the MTG engine. Your job is to walk
through the bugs documented in `audits/AUDIT_BUGS.md` and, for each
real engine bug, write a Rust test that **fails right now** because
the bug exists and **will pass** once the bug is fixed. **Do not fix
any bugs.** Your output is test files only.

The point of these tests is two-fold:

1. They give the next person to fix a bug a runnable, self-contained
   reproduction so they don't have to re-derive the failure mode from
   the prose entry.
2. Once a bug is fixed, the test stops failing without anyone having
   to remember the test exists. The test transitions from "this bug
   exists" to "this bug is regression-protected" automatically.

## Workflow

You're the only agent working on this. Work directly on `master` in
the main checkout at `/Users/dlaw/mtg/`. No worktrees, no UUID
prefixes.

### One-time setup at session start

1. Make sure master is clean and synced:
   ```bash
   cd /Users/dlaw/mtg
   git status                          # should be clean
   git fetch origin
   git pull --ff-only origin master    # no-op if already synced
   ```
2. Read `audits/AUDIT_BUGS.md` end-to-end so you have the full bug
   landscape in mind. The file is organized into **17 named families**
   (Subtype filter family, Damage helper bypass, Hexproof/protection,
   Token copy, Snapshot anthems, Auto-pick, Transform/DFC, etc).
3. Read `audits/AUDIT_TEST_PROGRESS.md` to see which bugs already
   have a test. Don't redo finished work. If the file doesn't exist
   yet, you're starting fresh — create it as part of your first
   commit. Format described below.

### Per-bug-test cycle

Run this loop for every bug. **One commit per bug** so the history is
easy to review and partial progress is recoverable.

1. **Pick the next undone bug from `AUDIT_BUGS.md`**. Skip the bugs
   listed in "Bugs you should skip" below. Work through the file in
   family order — that maximizes context reuse, since many bugs in a
   family share setup boilerplate.

2. **Verify the bug is real** before writing a test:
   - Read the file/line references in the bug entry. Files have not
     moved (`mtg-engine/src/...` is still where the bug entry says it
     is) but **line numbers in older bug entries may have drifted**;
     treat them as approximate anchors and grep for the symbol or
     comment text instead.
   - Look up oracle text for any cards involved using
     `python3 scripts/oracle_lookup.py lookup "Card Name"`. Do not
     trust your training data — the cache at `data/oracle_cache.json`
     is the source of truth.
   - If the bug requires a corner case you're not sure about (e.g.
     "creature with deathtouch deals damage from an activated ability —
     is `dealt_deathtouch_damage` set?"), construct a quick mental
     model from the actual code, not from your priors.
   - If after investigation the bug **does not exist** (the entry was
     speculative or the code has been refactored), record that in
     `AUDIT_TEST_PROGRESS.md` with status `not-reproduced` and a
     one-line explanation, then move to the next bug.

3. **Write the test**. Conventions in this repo:

   - **Engine bugs** → `mtg-engine/tests/audit_<family>_<topic>.rs`
     where `<family>` is a short slug for the family ("subtype",
     "damage_helper", "token_copy") and `<topic>` is optional. One
     file per family is fine; multiple files per family is also fine
     if a family is large. Look at the existing
     `mtg-engine/tests/audit_bugs.rs` and `audit_bugs2.rs` for the
     style — both are full of failing tests written for earlier
     bugs, and you should mirror their structure. You can either
     extend those existing files or create new ones; your call based
     on what reads better.
   - **Harness bugs in `mtg-player/src/llm.rs`** → inline
     `#[cfg(test)] mod tests` inside `llm.rs` itself, alongside the
     existing `format_counters_*` tests. Look at the
     `format_counters_ignores_slime_and_study` test for the pattern —
     **note that this particular test currently enshrines Bug 37-001
     and needs to be inverted as part of writing the new test**.
   - **Harness integration tests** → `mtg-player/tests/llm_<topic>.rs`.

   File header doc (for new files):
   ```rust
   //! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
   //! Each test is expected to FAIL until the corresponding bug is
   //! fixed. Once the fix lands the test transitions from "proves the
   //! bug exists" to "regression-protects against the bug coming back".
   //!
   //! Bugs covered in this file:
   //! - Bug XX: <one-line summary>
   //! - Bug YY: <one-line summary>
   ```

   Per-test header:
   ```rust
   /// Bug XX (audits/AUDIT_BUGS.md): <one-line summary>.
   ///
   /// Oracle: "..." (verbatim, from scripts/oracle_lookup.py).
   ///
   /// Failure mode: <one paragraph describing what the engine
   /// currently does and why it's wrong>.
   ///
   /// This test asserts the EXPECTED CORRECT behavior, so it
   /// currently fails. It will start passing as soon as Bug XX is
   /// fixed.
   #[test]
   fn bug_XX_short_descriptive_name() {
       // ... setup ...
       // ... action ...
       // ... assertion (encodes the correct behavior) ...
   }
   ```

   The test must:
   - Be **self-contained**. Set up exactly the conditions needed to
     trigger the bug. Use `mtg-engine/tests/common/mod.rs` helpers
     (`game_at_step`, `ready_creature`, `P0`, `P1`, etc).
   - **Assert the correct behavior** (the post-fix state), not the
     buggy behavior. The test fails today because the engine's actual
     behavior diverges from the assertion. The test starts passing
     once the engine is fixed.
   - Use `#[test]` plain — **do not** mark with `#[ignore]`. The
     existing `audit_bugs.rs` files use plain `#[test]`; the test
     suite is set up to allow these to fail.
   - Have a clear failure message via `assert!(condition, "...")` or
     `assert_eq!(actual, expected, "...")` so when the test eventually
     runs and fails, the error is self-explanatory and points back to
     the bug.

4. **Confirm the test fails for the right reason** before committing:
   ```bash
   cargo test --test audit_<family>_<topic> bug_XX_short_descriptive_name -- --nocapture
   ```
   The test must FAIL. Read the failure message. If it fails for an
   unrelated reason (panic, wrong setup, missing card in registry,
   etc.) fix the test until it fails because of the documented bug,
   then commit. **A test that doesn't fail proves nothing.** A test
   that fails for the wrong reason is misleading.

   For inline `mtg-player/src/llm.rs` tests:
   ```bash
   cargo test -p mtg-player <test_name> -- --nocapture
   ```

5. **Update `audits/AUDIT_TEST_PROGRESS.md`**. This file is a flat
   index of which bugs have tests. Append your entry to the bottom
   in this format:

   ```markdown
   - Bug XX-YYY (Family Name) — `mtg-engine/tests/audit_<family>_<topic>.rs::bug_XX_short_descriptive_name` — status: failing-as-expected
   ```

   Status options:
   - `failing-as-expected` — test is written, runs, and fails for
     the documented reason. Most common case.
   - `not-reproduced` — bug doesn't exist after investigation; no
     test written. Add a one-line note explaining what you checked.
   - `skipped-judgment-call` — for harness/UI bugs where the
     "correct" behavior is subjective. Add a one-line note.
   - `blocked` — writing the test requires infrastructure that
     doesn't yet exist. Add a one-line note describing the blocker.

6. **Commit and push**:
   ```bash
   git add mtg-engine/tests/audit_<family>_<topic>.rs audits/AUDIT_TEST_PROGRESS.md
   git commit -m "Tests: Bug XX (<one-line summary>)"
   git push origin master
   ```

7. **Loop back to step 1.**

## Bugs you should skip

- **`✅ FIXED` bugs** (Bug A, B, C, AJ, H1). They already have
  regression tests landed alongside the fix. Skip them; if the
  existing test is somehow incomplete that's a separate problem.
- **`P1`–`P5` past-fix records** and **`M1`–`M4` model behavior
  notes**. They're not engine bugs.
- **`H4` (Begin Combat prompts confuse the model)** — explicitly
  documented as covered by Bug H6 and is a model-prompt issue.
- **Bugs whose "Severity" is `low` AND whose proposed fix is "fix
  the wording / display"** — these are subjective and a test would
  encode arbitrary preferences. Use judgment.
- **Bugs that depend on infrastructure that doesn't yet exist**
  (e.g. anything requiring a 4+ player game harness, or anything
  requiring a working CR-correct stack-based trigger system if the
  engine still resolves activated abilities inline). Mark these
  `blocked` in the progress file with a note.

## Harness/UI bug judgment

For bugs in `mtg-player/src/llm.rs` and the surrounding harness:

- **Write a test** when the assertion is concrete and the expected
  behavior is unambiguous. Examples:
  - Bug 37-001 (`format_counters` hides Slime/Study): inline test
    asserting that `format_counters` with a Slime counter returns
    `Some("SLIMEx5")` or similar. Today the function returns `None`
    for Slime, which is the bug.
  - Bug 37-002 (target-selection prompts don't disambiguate): unit
    test that calls the target-selection prompt builder with two
    same-named creatures and asserts the output contains `#1` and
    `#2`.
  - Bug 76-001 (Skirsdag debug-format `{:?}` in label): assertion
    that the activation label string does NOT contain the substring
    `ObjectId(`.

- **Skip the test** when the bug is fundamentally about prompt
  clarity in a subjective way. Examples:
  - Bug H7 (target-choice and trigger-ordering prompts use the same
    opaque format) — "opaque" is subjective; mark
    `skipped-judgment-call`.
  - Bug H6 (BeginCombat prompts confuse the model) — same shape.

When in doubt, skip with a note rather than write a test that
encodes a preference. The next person can decide what "good" looks
like.

## Verifying the bug exists before writing the test

This is the most important step. Each bug entry contains a code
snippet and a "what oracle says vs what code does" section. Before
you write a test, **mentally trace the code path that would trigger
the bug**:

1. What sequence of actions creates the precondition (the right
   board state, the right cards in graveyard, the right player on
   priority)?
2. What single action triggers the buggy code path?
3. What does the engine actually do, and what should it do
   according to oracle text?

If you can't answer all three from reading the code and the oracle
text, the test you write will likely be wrong. Either:
- Read the relevant engine code (`mtg-engine/src/engine.rs`,
  `mtg-engine/src/state.rs`, `mtg-engine/src/sba.rs`, etc) until you
  can answer them, or
- Mark the bug `blocked` with a note describing what you couldn't
  resolve, and move on.

The test setup needs to match the precondition exactly. If you set
up the wrong board state and the test passes, you've "proved" a bug
exists when it doesn't, which is worse than no test.

### About the audit log file the bugs were mined from

`audits/AUDIT_BUGS.md` was originally produced by mining a tournament
log named `verify-draft-8seat-high-v5.log`. **That log file no longer
exists** — it was an untracked run artifact at the repo root that got
cleared during cleanup. About 7 bug entries cite specific line
numbers in that log under "Audit evidence:" or "Did fire / not fire".

Treat those citations as historical context, not actionable
references. You don't need the log file to write the tests:

- The bug entry already contains the code snippet, the oracle
  quotation, and the failure-mode description.
- The current source tree is the source of truth for whether the
  bug still exists. Spot-check the file:line refs in the bug entry
  against current code.
- If the bug entry says "fired in log line N" and you can no longer
  see line N, that's fine — the bug either still exists in the code
  (write the test) or it's been fixed (mark `not-reproduced`).
- Do NOT try to fetch, regenerate, or replay the log. It's gone and
  it's not blocking you.

The freshness of the audit pass also doesn't matter for test-writing.
Even if the v5 log was generated weeks ago, the bugs it surfaced are
still real if the current source tree still exhibits them — and your
verification step (mental code trace + spot-check the file/line
anchors) catches the cases where they don't.

## Oracle text discipline

Use `scripts/oracle_lookup.py` for every card you reference. Examples:

```bash
python3 scripts/oracle_lookup.py lookup "Olivia Voldaren"
python3 scripts/oracle_lookup.py lookup "Snapcaster Mage"
python3 scripts/oracle_lookup.py lookup "Geist of Saint Traft"
```

If a card isn't in the cache, fetch it:

```bash
python3 scripts/oracle_lookup.py fetch "Card Name"
```

Paste the **verbatim** oracle text into your test's docstring. Do
not paraphrase. Do not trust your training data. If the bug entry's
oracle quotation differs from the cache, trust the cache and update
the test (and consider whether the bug entry needs an update too,
though that's a separate task).

For tricky rules questions ("does deathtouch from non-combat damage
trigger SBA?", "what does CR 614.1c say about replacement effects?"),
you may search the web for the Comprehensive Rules section and quote
it verbatim in the test docstring. Stick to wizards.com and Scryfall
rulings; ignore random forum threads.

## Don't ask questions

You are running unattended. Do not stop to ask the human for
clarification. If you're unsure whether a bug is real, write the test
anyway and let it fail naturally — that's the whole point of these
tests. If the test passes (the engine is actually correct), the test
acts as a regression test going forward. Either way, more
information.

If you can't make progress on a bug after a reasonable attempt, mark
it `blocked` or `not-reproduced` and move on.

## Per-bug-test checklist (run before each commit)

- [ ] Test name is `bug_<XX>_<descriptive_name>` and matches the bug
      ID in `AUDIT_BUGS.md`
- [ ] Test docstring includes the bug's name, verbatim oracle text,
      and a one-paragraph failure-mode description
- [ ] Test asserts the EXPECTED CORRECT behavior, so it currently
      fails for the documented reason
- [ ] You ran `cargo test --test <file> <test_name>` and confirmed
      the failure message points back to the bug, not to a setup
      mistake or panic
- [ ] `cargo check` is clean (no new warnings)
- [ ] `audits/AUDIT_TEST_PROGRESS.md` has a new entry for this bug
- [ ] Commit message is `Tests: Bug XX (<one-line summary>)`
- [ ] You did NOT modify any source code in `mtg-engine/src/`,
      `mtg-player/src/` (except inline `#[cfg(test)] mod tests`
      blocks for harness bugs), or `mtg-draft/src/`
- [ ] You did NOT modify `audits/AUDIT_BUGS.md` itself
- [ ] You did NOT fix the bug

## Session-end checklist

- [ ] Last test has been pushed to master
- [ ] `audits/AUDIT_TEST_PROGRESS.md` reflects every test you wrote
      (and every bug you skipped, with reason)
- [ ] `git status` is clean
- [ ] `git rev-list --left-right --count master...origin/master` is
      `0	0`

## Files of interest

- `audits/AUDIT_BUGS.md` — the bug list. Read end-to-end at session
  start.
- `audits/AUDIT_TEST_PROGRESS.md` — the test-tracking index. You
  append to it as you go.
- `mtg-engine/tests/audit_bugs.rs`, `audit_bugs2.rs` — existing
  failing-test files. **Read both before writing your first test
  to absorb the style.**
- `mtg-engine/tests/common/mod.rs` — shared test helpers (`P0`,
  `P1`, `game_at_step`, `ready_creature`, etc). Use these instead
  of re-implementing.
- `mtg-engine/src/cards/isd/*.rs` — card implementations. Read the
  card behind each bug to understand the trigger surface.
- `mtg-engine/src/engine.rs`, `state.rs`, `sba.rs`, `triggers.rs`,
  `combat.rs`, `destruction.rs` — the engine paths most bugs touch.
- `mtg-player/src/llm.rs` — harness bugs live here. Inline
  `#[cfg(test)] mod tests` is the existing test convention.
- `scripts/oracle_lookup.py` — oracle text source of truth.
- `data/oracle_cache.json` — local oracle cache.
