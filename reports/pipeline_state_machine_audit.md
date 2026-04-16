# Pipeline State Machine Audit

Audit of `pipeline/cli.py` — specifically, the ticket lifecycle, the
commands that drive transitions, and the edge cases they miss.

The pipeline coordinates four agents (auditor, test-writer, fixer,
dedup) and the human. Each ticket tracks one suspected bug through a
finite-state lifecycle. This document enumerates the states, the
allowed transitions, and the gaps I found.

---

## 1. States (ticket `status:` values)

| Status           | Kind      | Meaning                                                  | Worktree? |
|------------------|-----------|----------------------------------------------------------|-----------|
| `new`            | open      | Auditor wrote it; waits for test-writer                  | no        |
| `tested`         | open      | One or more failing tests exist and compile              | yes       |
| `fixed`          | open      | Fix committed, all tests pass, awaits human merge        | yes       |
| `fix_failed`     | open      | Fixer gave up after max attempts                         | yes (kept for inspection) |
| `shipped`        | terminal  | Merged into `master`                                     | no        |
| `false_positive` | terminal  | Test-writer could not write a failing test               | no        |
| `closed`         | terminal  | Absorbed by a merged-\* parent, or abandoned by human    | no        |

`closed` carries a required `closed_reason`: `absorbed` (dedup / consolidate) or `abandoned` (manual).

Per-test substatuses `confirmed` / `rejected` / `blocked` live in the
test-writer's staging JSON; Python aggregates them into one ticket
status. They are not persisted to the ticket frontmatter.

## 2. Transitions

```
                 ┌──── retry --to new ────┐
                 │                        ▼
   audit ─► new ─test─► tested ─fix─► fixed ──merge──► shipped
             ▲            ▲              │
             │            │              └─► fix_failed
             │            │                   │ │
             │            └──── retry --to tested
             │                                │
             └── retry (when status=new)  ◄───┘
             └── retry --force (from false_positive)

   any open status ──► consolidate ──► closed (absorbed)
   any status       ──► close       ──► closed (abandoned)

   test (blocked)   ──► new  (allow_engine_edits=true)
   test (any rejected/none confirmed) ──► false_positive

   consolidate may also mark a ticket `tested` directly when it
   inherits a worktree + fully-covered tests from a single tested source.
```

## 3. Transition table

| From \ Event | audit | test | fix | merge | retry --to new | retry --to tested | consolidate | close | dedup |
|---|---|---|---|---|---|---|---|---|---|
| (none) | → new | — | — | — | — | — | — | — | — |
| new | — | → tested / new⁺ / false_positive | ❌ | ❌ | → new (no-op-ish) | ❌ (no tested_sha) | → closed | → closed | via consolidate |
| tested | — | ❌ | → fixed / fix_failed | ❌ | → new | → tested (reset to recorded sha) | → closed | → closed | via consolidate |
| fixed | — | ❌ | ❌ | → shipped | → new (wipes fix!) | ❌ (refuses) | **rejected** | → closed | **rejected** |
| fix_failed | — | ❌ | ❌ | ❌ | → new | → tested | **rejected** | → closed | **rejected** |
| shipped | — | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **silently overwrites** (bug) | ❌ |
| false_positive | — | ❌ | ❌ | ❌ | → new (requires --force) | → tested (requires --force) | ❌ | → closed (silently overwrites) | ❌ |
| closed | — | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **silently overwrites** (bug) | ❌ |

⁺ `new` here means the "needs_engine" escape hatch: test-writer flagged at least one blocked scenario, so the ticket is rewound to `new` with `allow_engine_edits: true` set.

❌ is enforced by Python with an exit-1 error.

---

## 4. Gaps and bugs

### 4.1 Ticket-state API is not centralized

Some transitions use `update_ticket_status(tid, status, extra_fm)`,
which Python designed to be the single entry point (it writes
frontmatter, then archives or unarchives based on terminality). Other
transitions bypass it:

- `cmd_merge` writes frontmatter directly and calls `archive_ticket`
  by hand (cli.py:1687–1692).
- `cmd_retry` writes frontmatter directly and calls
  `unarchive_ticket` by hand (cli.py:1921–1927). It also has a
  special-case comment acknowledging the inconsistency.
- `cmd_consolidate` writes closed-reason frontmatter directly
  (cli.py:2440–2451).
- `cmd_close` writes frontmatter directly (cli.py:2620–2628).

This is how bugs 4.2 and 4.3 below slip in.

### 4.2 `close` silently overwrites terminal tickets

`cmd_close` sets `status=closed`, `closed_reason=abandoned`,
`closed_at=now` on ANY ticket, with no check on the current status. A
shipped ticket can be "un-shipped" to abandoned; a closed-absorbed
ticket can be rewritten as closed-abandoned (losing the original
`absorbed_into`). Neither is intentional.

### 4.3 `merge` never runs its own post-merge test check

cli.py:1653–1665 verifies the fix by running `cargo test -- test_name`
IF `fm.get("test_name", "")` is non-empty. But nowhere in `cmd_test`
does Python write a `test_name` frontmatter field — the test phase
records `test_file`, `tests_confirmed`, `tests_total`, and per-test
state in the body, but no `test_name`. So this check ALWAYS no-ops.
Merge trusts validate_fix.sh's earlier pass, which ran before any
parallel fixes landed.

This means two independent `fix` → `merge` sequences on conflicting
engine paths can both succeed, with the second one breaking the
first's tests and nothing catching it until the next full-suite run.

### 4.4 Retry from `fixed` wipes the fix silently

`cmd_retry` defaults `target` to `new` for any status not listed
specially. A `fixed` ticket is not listed, so `retry fixed-ticket`
falls through to `target=new`, which `git reset --hard master`s the
worktree branch. The validated fix is gone, with only a
`## Attempt N` section as evidence.

Fix: explicitly refuse to retry from `fixed` without a confirmation
flag (same rule as `false_positive`), or default to `--to tested`.

### 4.5 Test phase treats protocol failures as `false_positive`

In `cmd_test`, if every retry attempt fails validation (agent error,
malformed staging, field-level protocol errors, coverage gaps, dirty
worktree), `aggregate` remains `false_positive` (initialized on line
1027). The ticket is then transitioned to `false_positive`, which is
a terminal status that requires `--force` to retry.

A rigorous false-positive verdict is "tests were written, all
compiled, all passed against current code." An agent that 3-times
errored on JSON shape has reported nothing about the underlying bug.
Those should land in a distinct `test_failed` / retryable state, or
at least not claim "false positive" to the operator.

### 4.6 `allow_engine_edits` is never cleared

Once a blocked test sets `allow_engine_edits: true`, the flag lives
on the ticket forever. A later retry or fix phase will read it back.
Not a correctness bug today (only test-writer reads it) but a latent
surprise.

### 4.7 Worktree lifecycle has implicit invariants

- `cmd_fix` refuses if the worktree doesn't exist. Status=tested
  guarantees worktree existence ONLY on the code path that came from
  `cmd_test`. A ticket reaching `tested` via `cmd_consolidate`
  inheritance has its worktree renamed from a source. A ticket reset
  to `tested` via `retry --to tested` has a worktree rebuilt from
  scratch. If any of these paths fail midway, the invariant breaks.
- `remove_worktree` is called on false_positive, merge, close,
  retry --to new. But NOT on consolidate-absorb for non-tested
  sources. Absorbed `new` sources never had a worktree; absorbed
  `tested` sources get theirs renamed, which is right — but the
  `absorbed_into=<parent>` closed children still carry a `worktree:`
  field pointing at the ORIGINAL ticket's directory (cli.py:2449
  pops it only for the tested source). A coordinator reading
  `absorbed` ticket frontmatter sees a stale worktree pointer. Not
  load-bearing, but confusing.

### 4.8 Dedup slug uniqueness is not checked across files

Dedup produces N staging files. Per `load_consolidation_staging`,
each file's `slug` must be kebab-case but there's no check that the N
slugs are distinct. The agent prompt asks for distinctness but Python
doesn't validate. Two files with the same slug → two consolidations
mint `merged-<slug>-01` and `merged-<slug>-02` respectively. That's
accidentally OK (they both get unique IDs), but the slug collision
muddies reporting.

### 4.9 `insights` write-back is racy

`cmd_audit` runs in a ThreadPoolExecutor with N workers. Each worker
appends to `pipeline/prompts/auditor-insights.md` with `open(path, "a")`
(cli.py:876–879). Concurrent appends to the same file are usually
atomic for small writes, but interleaved line-by-line writes can
corrupt entries. More seriously, an auditor reading this file at the
same time another worker is appending can see partial content. Easy
fix: a threading.Lock, or write to a staging append-log that's
collated post-run.

### 4.10 Metrics coverage is uneven

`runs.jsonl` is appended for audit, test, and fix. `findings.jsonl`
is appended for ticket creation, test_*, fix_*, and shipped events.
But:
- `cmd_retry` writes neither.
- `cmd_close` writes neither.
- `cmd_consolidate` writes neither (no event for "absorbed").
- `cmd_dedup` writes none at the dedup level (individual consolidations
  write nothing as above).

The "lifecycle" view in `metrics.py` is therefore blind to every
state transition that isn't driven by an agent subprocess.

### 4.11 JSON parsing is duplicated 4x

`load_audit_staging`, `load_test_staging`, `load_fix_staging`,
`load_consolidation_staging` all follow the same pattern:
`_load_json` → per-field `_require`. Only the schemas differ. No
shared schema description.

### 4.12 Retry loops are duplicated 4x

`cmd_audit`, `cmd_test`, `cmd_fix`, `cmd_dedup` each embed a
`for attempt in range(1, MAX):` loop with:
1. Build a prompt with a trailing `retry_note`.
2. Run the agent.
3. Check `is_error`.
4. Check staging file exists.
5. Parse staging → StagingError.
6. Run validators.
7. On failure, compose a retry note and `continue`.

Each loop has small idiosyncratic differences (the fixer checks a
shell validator; the test-writer has 3 distinct retry reasons; the
auditor tolerates empty-findings pass) but the skeleton is identical.

### 4.13 Prompt composition is split between files and Python strings

The "shared" prompt per role lives in `pipeline/prompts/*.md`. The
"per-agent" block (ticket body, test file path, staging path, retry
note) is built as a Python f-string inside each cmd_*. Consequences:
- Changing a prompt requires editing both an external file and Python.
- The per-agent strings contain schema reminders that duplicate the
  shared prompt (e.g., `cmd_audit`'s per_agent repeats that
  `engine_path` is an array).
- The Python literal style makes it hard to diff prompt edits.

### 4.14 `list_tickets()` includes archive

`list_tickets(status=None, card=None)` reads from active AND archive.
Called by `cmd_tickets` without filters, it prints every ticket ever
created (hundreds). The user probably wants "active tickets" by
default, with a `--all` to include archive.

### 4.15 `cmd_fix` can only run on a single ticket

There's no `--tickets` plural flag and no parallelism argument.
`cmd_audit` and `cmd_test` support `--parallelism`. Practically
nothing is stopping `cmd_fix` from supporting the same pattern (each
ticket has its own worktree), so this is a missing feature rather
than an invariant.

### 4.16 Dry-run semantics are inconsistent

- `audit --dry-run`: prints plan, exits before fetching oracle texts
- `test --dry-run`: prints plan, exits before spawning
- `fix --dry-run`: prints plan, exits before spawning
- `consolidate --dry-run`: actually DOES more work (validates
  referenced tickets, determines parent id, prints summary) and also
  exits without writing
- `dedup --dry-run`: only prints spawn intent

### 4.17 `cmd_fix` takes `--ticket` singular, `cmd_test` takes `--tickets` plural

Minor UX inconsistency. A user running `./cli.py fix --tickets a,b` gets
an argparse error; running `./cli.py test --ticket a` gets the same.

### 4.18 `cmd_audit`'s `--cards` uses comma OR semicolon splitting

`sep = ";" if ";" in args.cards else ","` (cli.py:666). This is a
workaround because card names contain commas ("Mikaeus, the Lunarch").
Users have to know to reach for a semicolon when that happens. A JSON
array or an explicit `--card` flag (repeatable) would be clearer.

### 4.19 Per-ticket `test_file` is captured in frontmatter but never used except in `fix`

The field is written at test time, but then `cmd_merge` hard-codes a
separate mechanism (the unused `test_name`). If test_file were used
consistently, the merge sanity-check in 4.3 would be fixable.

### 4.20 Retry does not log a retry event

A human reading metrics or ticket history has no trace of "this
ticket was retried N times." The `## Attempt N` body section records
it qualitatively but no counter lives in frontmatter, and no event
lands in the jsonl logs.

### 4.21 `remove_logs_for_ticket` does substring matching

`LOGS_DIR.glob(f"*{ticket_id}*")`. Ticket `olivia-01` matches logs
from `merged-olivia-01-side`, etc. Low probability in practice but
incorrect.

### 4.22 `test_name` is misidentified in several places

- cli.py:1278 logs `test_result: aggregate` as `aggregate` string,
  but `findings.jsonl` consumers may expect `"tested"` / `"needs_engine"`
  / `"false_positive"` — those values aren't documented anywhere.
- `test_result` in `metrics.py` and `runs.jsonl` is typed differently
  across commands (auditor: None; test-writer: aggregate string;
  fixer: None).

### 4.23 `_parse_tests_section` is fragile regex

It uses `##\s+Tests\n` as the anchor and `(?=\n##\s|\Z)` as the end.
A ticket body whose `## Tests` section is followed by a section at
the very end without a blank line separator can confuse it. Worse,
if the ticket never went through audit (e.g., manually-crafted
merged-\* parent with different body style), the parser silently
returns `[]`.

### 4.24 Agent settings file path is relative to project root

`pipeline/agent-settings.json` is passed to `claude --settings ...`
with `str(agent_settings)` — but `agent_settings.exists()` uses
`PIPELINE_DIR / "agent-settings.json"`, which is absolute. The string
passed to the subprocess is then absolute too, so it works. But the
"deny access to archive" rule in that file uses `Read(pipeline/tickets/archive/**)`
— a PROJECT-RELATIVE glob. The test/fix agents run inside worktrees
where `pipeline/tickets/archive/` is a different physical directory
than the main repo's. If a worktree happens to lack an archive dir
(most do), the deny list is effectively a no-op for worktree agents.

### 4.25 Consolidate's `also_closes` and per-test `source_ticket` overlap check

`cmd_consolidate` refuses a consolidation where the same ticket
appears in both `also_closes` and a per-test `source_ticket`. Good.
It also refuses duplicates within `also_closes`. Good. It does NOT
refuse duplicate per-test `source_ticket` values pointing at the
same ticket — that's intentionally allowed (one child can contribute
multiple tests).

But coverage validation uses `_required_counts` per child, meaning
the parent must have ≥ N tests attributable to child X if child X
had N tests. The current logic includes tests with `source_ticket:
(new)` under the parent's "own" id if the child happens to have no
prior id — this is a heuristic that can over-fire when the agent
legitimately writes fresh tests whose scenarios are tangential to a
closed child's original test set.

### 4.26 Staging directory is shared mutable state

`STAGING_DIR = pipeline/staging/` is used:
- by `cmd_audit` for per-card run output (`<run_id>.json`)
- by `cmd_dedup` for agent's consolidation files
- NOT by test/fix (those use worktree-local staging)

A parallel `audit` + `dedup` could collide if their run_ids happened
to match a `consolidation-*.json` name (they don't — naming
conventions are disjoint — but there's no enforcement).

Additionally, `dedup` snapshots `preexisting` files at the start and
treats anything new as agent output. If another process writes to
STAGING_DIR during the run, dedup can pick up stranger's files.

### 4.27 No concurrency protection on ticket files

Multiple parallel workers can call `update_ticket_status` on
different tickets safely (each writes its own file). But
`update_ticket_status` reads-then-writes. A second concurrent writer
to the SAME ticket would race. Today's design makes this rare
(`audit` writes new tickets with unique ids; `test`/`fix` own one
ticket each; `retry`/`close`/`consolidate` are human-driven one-ats).
Still worth noting.

### 4.28 Ticket ID allocation races

`cmd_audit` computes `next_num` by globbing existing tickets for
this card snake. Two parallel audits of the same card (not how it's
called today, but no lock prevents it) would both see the same max
and both pick the same next_num. First writer wins, second clobbers.
Same issue in `cmd_consolidate`'s merged-\* numbering.

### 4.29 No "needs-human" status

The only way to signal "this ticket is stuck on something only a
human can unstick" is `fix_failed`. That lumps together:
- fixer exhausted retries with a reasonable post-mortem
- fixer crashed 3x without ever producing a diff
- test-writer found a real bug but couldn't express it
- human gave up and manually set the status

No way to distinguish "ready for human triage" from "shipped garbage"
at a glance.

### 4.30 Status migrations

`scripts/migrate_statuses.py` exists as a one-off script for a
previous status-vocabulary change. It's dead weight now.

### 4.31 Gitignored staging + worktree rules assume perfect cleanup

If an agent's subprocess is killed partway through (SIGTERM from
parent, OOM, etc.), its worktree may be left with dirty state. The
main process has no cleanup pass. `ensure_worktree` happily returns
an existing dirty directory on next run.

---

## 5. Code-shape issues

### 5.1 File size

2752 lines in one file. Split roughly:
- 520 lines: orchestration helpers (worktree, agent runner, frontmatter)
- 150 lines: staging JSON loaders
- 270 lines: cmd_audit
- 380 lines: cmd_test
- 230 lines: cmd_fix
- 230 lines: cmd_retry
- 300 lines: cmd_consolidate
- 140 lines: cmd_dedup
- 150 lines: cmd_merge / cmd_close / cmd_tickets / cmd_show / cmd_status
- 150 lines: cmd_report
- 120 lines: argparse wiring + misc
- remainder: comments, blank lines

### 5.2 Duplication by theme

- **Prompt construction** (4 call sites, ~200 LOC total): each cmd_*
  builds a per-agent string from an f-string template. The templates
  are similar: a "ticket-body" block, a "staging-output" block, a
  "ticket-id/test-file" block, sometimes a "retry note" block.
- **Retry loop** (4 call sites, ~120 LOC total): see 4.12.
- **Staging parser** (4 call sites, ~80 LOC total): see 4.11.
- **Jsonl logging** (9 call sites): same dict shape with different
  values; could be one helper that takes a role + outcome + result.
- **Frontmatter clearing on retry**: hard-coded list of keys. A
  single "phase" abstraction (each phase owns its own fm keys) would
  centralize this.
- **Ticket status update + archive/unarchive**: 4 transitions skip
  `update_ticket_status`, each replicating the logic. See 4.1.

### 5.3 Unused / underused

- `run_agent` wraps `run_agent_in` with `cwd=PROJECT_ROOT`. Only
  `cmd_audit` and `cmd_dedup` use it (project-root context). All
  others use `run_agent_in` directly.
- `_text_similarity` is defined but never called.
- `merge_worktree` is defined (cli.py:199) but `cmd_merge` uses an
  inline `subprocess.run(["git", "merge", ...])` instead.
- `card_to_snake` could just be `re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")`.

### 5.4 Comments that restate code

Most block comments at the top of each cmd_* do describe non-obvious
semantics (good), but many inline comments re-narrate what the code
immediately does:
- `# Parse JSON staging` followed by `if not staging_file.exists()`
- `# Per-test validation` followed by `for t in per_test: ...`
These can be removed.

---

## 6. Recommendations (for the refactor to follow)

1. **One transition function.** All status changes go through
   `transition(ticket_id, *, to_status, extra_fm, close_reason=None,
   reset_worktree=False)`. This function owns the active↔archive
   move, the worktree cleanup, and the metrics log.

2. **External prompt templates.** Per-agent prompt bodies become
   `prompts/<role>.peragent.md` with `{placeholders}`. Python
   `.format_map(ctx)` instead of f-strings. The shared `prompts/<role>.md`
   loses its inline schema examples (which are duplicated in
   Python's staging loaders).

3. **One schema-driven loader.** `load_staging(path, schema)` where
   schema is a small dataclass (`field_name: type`, optional
   validators). Replaces the four `load_*_staging` functions.

4. **One retry wrapper.** `run_with_retry(*, prompt, cwd, validate)`
   where `validate(result, staging_path) -> (ok, retry_note)`.
   Replaces the four hand-rolled loops.

5. **One jsonl helper.** `log_run(role, ticket_id, result, outcome, notes)`.

6. **Kill dead code.** `_text_similarity`, `merge_worktree` (or use
   it from cmd_merge), `scripts/migrate_statuses.py`, the unused
   `test_name` frontmatter path in cmd_merge.

7. **Fix retry-from-fixed.** Require explicit `--force` or default
   to `--to tested` for fixed status.

8. **Fix close-from-terminal.** `cmd_close` should refuse if the
   ticket is already terminal.

9. **Actually run the post-merge test.** Parse Implementation lines
   from `## Tests` to get the test names; run `cargo test -- <names>`
   before declaring `shipped`.

10. **Lock the insights write.** A threading.Lock around the append,
    or write per-worker insight files and collate at run end.

11. **Clear `allow_engine_edits` on success.** When a ticket reaches
    `tested`, drop the flag.

12. **Log retries and closes.** `runs.jsonl` gains `retry` and
    `close` rows for full lifecycle visibility.

13. **List only active tickets by default.** `cmd_tickets` adds
    `--all` for archive inclusion.

14. **Refuse absorbed write-overs.** `archive_ticket` errors if it
    would overwrite an existing archive entry.

After applying these, the target file count is:
- `pipeline/cli.py` — argparse + dispatch, ~80 lines
- `pipeline/pipeline/state.py` — statuses, transitions, ticket I/O, ~180 lines
- `pipeline/pipeline/agent.py` — subprocess + retry wrapper + stream log, ~180 lines
- `pipeline/pipeline/staging.py` — schemas + loader, ~120 lines
- `pipeline/pipeline/commands.py` — one function per command, ~500 lines
- `pipeline/pipeline/report.py` — metrics dashboard, ~150 lines

Total: ~1200 lines across modules, or ~900 if we merge state/agent
into one orchestration module. Single-file target: under 1000 lines
is achievable with these changes.
