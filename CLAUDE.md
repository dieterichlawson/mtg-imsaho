# CLAUDE.md

## Git workflow

Break all changes down into a series of small commits when possible. Each commit should represent one logical change (a bug fix, a feature addition, a refactor, etc.) so that individual changes are easy to review and revert.

## Code quality

Always fix compiler warnings before finishing work. Run `cargo check` and ensure zero warnings.

## Verifying test results

When running `cargo test`, check for BOTH test failures AND compilation errors. A compilation error in one test file prevents that binary from running, which shows as 0 failures — because the tests never executed, not because they passed. Always report compilation errors as problems, not as passing tests. If the numbers don't make sense, investigate before reporting success.

Do NOT pipe `cargo test` through `grep "FAILED"` as a way to check test results — this silently drops compilation errors. Instead, check the exit code first (`cargo test; echo $?`), and if non-zero, look at the full output for both "FAILED" and "could not compile" lines.

## Player-facing changes: one decision, three surfaces

Every decision the engine asks for is presented three times, and they are
separate code:

- `mtg-player/src/cli.rs` — the screen a person reads and types at.
- `mtg-player/src/llm.rs` — the prompt text and the JSON response schema an
  LLM seat answers through.
- `mtg-player/src/random.rs` — the seat the invariant fuzzer plays, which is
  how most of the engine gets exercised at all.

**Changing what the engine asks means changing all three.** A new
`ResolutionChoiceKind`, a changed `min`/`max`, a prompt that used to be
asked twice and is now asked once — each seat has to be walked, not just
the one you were looking at. The failure mode is quiet on every side but
the one you tested:

- The CLI shows a screen; the LLM seat gets a schema the API rejects with a
  400 before the model reads it, and the harness turns that into an empty
  answer indistinguishable from declining (#398 — a `claude -p` seat could
  not cast Skaab Goliath at all).
- The CLI and the LLM seat both work; the random seat answers with the
  minimum, which for an "up to N" slot is *nothing*, so the fuzzer stops
  reaching a whole class of resolution and the oracle goes quiet without
  failing.

Two rules follow, and both have been broken by changes that looked local:

1. **Never key a top-level JSON-schema property by a card name or any
   runtime string.** The API checks top-level keys against
   `^[a-zA-Z0-9_.-]{1,64}$`. Use an index array (`mark_indices`,
   `choose_card_set`) or nest the keys a level down. There are two request
   paths, not one: `mtg-player/src/llm.rs` for the game and
   `mtg-draft-runner/src/llm_client.rs` for the draft, which is a separate
   copy that has already missed one fix (#404). Both assert this rule.
2. **Never let a non-interactive seat answer with a constant** where the
   constant is a legal no-op. Roll it, or the fuzzer covers nothing.

## Player-facing changes: fit, and the size of a question

Two properties the interactive surface is expected to hold, both of which
have been broken repeatedly (#318, #350, #351, #352, #364, #365, #366):

- **Everything printed fits.** A row wider than its pane is wrapped or
  clipped deliberately, never printed over the border into the next pane.
  A list longer than the screen pages, and the pager can reach its last
  entry. Pages are sized in *rendered lines*, not in entries, or a page of
  wrapped rows scrolls its own heading away.
- **No question grows faster than the board.** A prompt offering one row
  per way of filling it — every subset, every pair, every mode-and-subset —
  is `C(n,k)` or `|a| x |b|` rows, which is an unreadable menu for a person
  and a token flood for a model. Ask for the set on one screen, or ask for
  one slot at a time. `mtg-engine/tests/prompt_shapes.rs` sweeps the card
  pool for requirements that would reintroduce this.

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
