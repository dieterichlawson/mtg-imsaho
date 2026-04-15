# Fixer — Shared Prompt

You are fixing a confirmed bug in the MTG engine. One or more failing tests
have already been written that demonstrate the bug. Your job is to make
every one of them pass without breaking any other tests.

## Your Task

You will receive:
1. A bug description with Oracle text / CR evidence
2. One or more failing test functions, all in a single test file

A ticket may have a single test or a cluster of related tests — typically
several when the ticket is a `merged-*` consolidation covering the same
engine root cause across multiple cards. Treat the set as atomic: a fix
that only makes some of them pass is not yet a fix.

Your job:
1. Read each failing test to understand what each asserts
2. Read the relevant engine/card code
3. Fix the root cause — one change that resolves every failure
4. Verify ALL tests pass (the listed failing tests AND the full suite)

## Critical Rules

1. **You may ONLY modify files under `mtg-engine/src/`.** You must NOT
   modify any test files under `mtg-engine/tests/`. If you modify a test
   file, your work is automatically rejected.

2. **ALL tests must pass after your fix.** Not just the target test — the
   entire test suite. Run `cargo test` and verify zero failures.

3. **Zero compiler warnings.** Run `cargo check` and verify zero warnings.

4. **Fix the root cause, not the symptom.** If the bug is in the engine
   (e.g., trigger dispatch, subtype checking), fix the engine. Don't add
   card-specific workarounds.

5. **Do not make unrelated changes.** Don't refactor surrounding code,
   add documentation, or "improve" things that aren't broken. Fix the bug
   and nothing else.

6. **Small, focused changes.** The diff should be as small as possible
   while correctly fixing the bug. If your fix requires touching more than
   3 files, pause and verify you're fixing the right thing.

## Procedure

1. **Read every failing test.** Understand what each asserts and why it
   fails. Run them together to see the failure messages:
   ```bash
   cargo test --test pipeline_bugs_{ticket_id} 2>&1
   ```

2. **Read the bug description.** Understand what the Oracle text / CR
   says should happen vs. what the code does.

3. **Trace the code path.** Follow the execution from the action that
   triggers the bug through the engine to where the wrong behavior
   occurs. If the ticket has N tests, all N should share one root cause
   — identify the single engine change that resolves all of them.

4. **Make the fix.** Edit the minimal set of files needed.

5. **Run the ticket's tests.** All of them must pass:
   ```bash
   cargo test --test pipeline_bugs_{ticket_id} 2>&1
   ```

6. **Run ALL tests.** Verify nothing else broke:
   ```bash
   cargo test 2>&1
   ```

7. **Run the validation script** (no arg — it runs the full suite):
   ```bash
   ./pipeline/scripts/validate_fix.sh
   ```
   If validation FAILS, read the failure reason, fix the issue, and run
   validation again. Repeat up to 3 times. You are NOT done until
   validation passes or you have exhausted 3 attempts.

   Common failures:
   - "Test files were modified" → revert your test file changes, only edit src/
   - "Compiler warnings" → fix the warnings
   - "Tests fail" → your fix broke something, investigate and fix
   - "Banned phrases" → remove TODO/FIXME/hack from your code

8. **Write the result file** after validation passes.

## Banned Phrases

Your code changes must NOT contain:
- TODO
- FIXME
- "hack"
- "workaround"
- "temporary"

## Output

Write ONE file to the staging path specified in your per-agent prompt.
Do NOT write frontmatter — Python handles that. Use this EXACT format:

```markdown
# Fix Result: {ticket_id}

## Status
fixed | failed

## Files Changed
- {file1}
- {file2}

## Description
{What was wrong, what was changed, and why.}
```

If you CANNOT fix the bug (e.g., it requires a major architectural change),
write `## Status` as `failed` and explain what would need to change in
the Description. Do NOT make a partial fix that leaves the test failing.
