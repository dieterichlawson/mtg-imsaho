# CLAUDE.md

## Git workflow

Break all changes down into a series of small commits when possible. Each commit should represent one logical change (a bug fix, a feature addition, a refactor, etc.) so that individual changes are easy to review and revert.

## Code quality

Always fix compiler warnings before finishing work. Run `cargo check` and ensure zero warnings.

## Verifying test results

When running `cargo test`, check for BOTH test failures AND compilation errors. A compilation error in one test file prevents that binary from running, which shows as 0 failures — because the tests never executed, not because they passed. Always report compilation errors as problems, not as passing tests. If the numbers don't make sense, investigate before reporting success.

## Repository layout

Keep the repo root tidy. When creating a new file, place it in the correct directory instead of at the root:

- `prompts/` — one-off or reusable prompt scaffolds (`*_PROMPT.md`) handed to agents.
- `docs/plans/` — planning documents, experiment designs, card-set plans, exemplar lists.
- `docs/` — longer-form design/reference docs that are neither plans nor reports.
- `reports/` — bug reports, verification reports, and any generated analysis intended to be read later.
- `audits/` — the running audit pipeline (`AUDIT_BUGS.md`, `AUDIT_PROGRESS.md`, agent runs, classification).
- `logs/` — any run output that should be kept. Prefer a dated subdirectory (e.g. `logs/overnight-smoketest/`).
- `scripts/` — shell/python helpers invoked by humans or cron.

Files that legitimately live at the root: `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `AGENT_COORD.md`, `TODO.md`, `.gitignore`, top-level crate directories, and the existing `data/` and `decks/` fixtures.

## Run output and logs

- `*.log` and `/results.json` are gitignored. Don't commit them.
- Never dump draft/verify logs at the repo root. If a script writes logs, point it at `logs/<run-name>/` and create the directory if missing.
- Delete obsolete run output as soon as you're done with it — don't let it pile up.
- If you generate a one-off report or notes file during a task, put it under `reports/` (or `docs/plans/` if it's forward-looking) rather than the root.
