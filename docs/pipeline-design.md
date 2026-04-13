# Bug-Finding & Fixing Pipeline — Design Document

This document captures the full design conversation and decisions for the
automated bug-finding and fixing pipeline. It should be kept up to date as
the pipeline evolves. Written so a fresh context can pick up where we left off.

## Problem Statement

The MTG engine has ~270 implemented Innistrad cards and a complex rules engine
(~5k lines in engine.rs, plus stack resolver, trigger system, combat, etc.).
Previous audit runs found 204 issues across 136 cards, but:

1. **Bug count never converges.** Every audit run surfaces ~100 more issues.
2. **Many issues are duplicates** of the same ~30-50 engine root causes, reported
   per-card. The raw count is misleading.
3. **Fix throughput is bottlenecked.** Of 87 tracked bugs in AUDIT_BUGS.md,
   only ~6 were fixed before the user did a push to fix all of them.
4. **Fixes reveal masked bugs.** Bug B was invisible until Bug A was fixed.
5. **Agents are unreliable.** They sometimes don't listen to prompts, defer
   work ("would need further investigation"), claim they can't write tests
   without engine changes, and produce false positives.

## Design Philosophy

### What Works (from previous audit experience)

- **Pre-fetched oracle text from Scryfall** as single source of truth.
  Agents are forbidden from using training-data memory for oracle text.
  This eliminated a major class of false positives.
- **"Quote both sides exactly" rule.** When claiming a mismatch, agents
  must quote both the oracle text and the code. Hallucinated bugs evaporate
  when this is enforced.
- **Structured report format.** Parseable output with required sections
  (Status, Code Issues, Tricky Interactions, Test Coverage).
- **AI game log mining.** Bugs found by mining real game logs (120k-line
  draft tournaments) are uniformly real — they actually happened in a game.
  Higher signal than code-reading audits.

### What Doesn't Work

- **Prompt-only discipline.** Telling agents "don't modify tests" or
  "don't defer work" via prompt alone — they ignore it under pressure.
  Constraints must be mechanical (file access, compilation gates).
- **Unbounded bug-finding tasks.** "Find bugs in this code" never terminates.
  "Make this failing test pass" does.
- **Parallel fix agents.** Multiple agents fixing overlapping code causes
  churn and regressions. Serial fixes are safer.
- **Agents as final reviewers.** Code review by an agent is weakly enforced.
  Human review of small diffs is more reliable and not much slower.

### Key Design Decisions

1. **No YAML specs, no DSL, no new test framework.** Plain Rust tests using
   existing `tests/common/mod.rs` helpers. No infrastructure to build or
   maintain. Agents write standard Rust test code.

2. **Mechanical enforcement over prompt discipline.** Each agent has:
   - File access restrictions (what they can write to)
   - A binary pass/fail gate (compilation, test results)
   - Banned output patterns (TODO, FIXME, etc.)

3. **Queue-based pipeline committed to git.** Each phase produces files in
   a queue directory. The next phase reads from that queue. Phases run
   independently — not all agents need to be running at once.

4. **Three-agent pipeline:** Auditor -> Test Writer -> Fixer.
   - Reviewer is skipped — the test writer's "must fail" gate IS the review.
   - Verifier is skipped — human review of the fixer's diff is the gate.

5. **Human review at two points:**
   - Test Writer output (is this test actually testing the right thing?)
   - Fixer output (is this diff correct?)
   In practice, the user can batch-review or spot-check.

## Approaches Considered and Rejected

### YAML Spec Files + Runner
Proposed: Write per-card specs in YAML, build a loader + executor.
Rejected because: building the schema/loader/executor is building a scripting
language. Each new mechanic needs schema extensions. Refactor-hostile (runtime
errors vs compile-time). No debugger. Existing test helpers already do this.

### Rust DSL / Scenario Builder
Proposed: A `Scenario::new().p1_hand(["Giant Growth"]).build()` builder.
Rejected because: `tests/common/mod.rs` already has `game_at_step()`,
`spell_in_hand()`, `castable_spell()`, etc. The "DSL" was reinventing
existing helpers with slightly different ergonomics. No new abstraction needed.

### Fuzzer with Invariants
Status: Not rejected, deferred. Still worth building as a parallel track.
A random-legal-move fuzzer with structural invariants (zone consistency, unique
IDs, legal_actions round-trip) would catch a different class of bugs than the
audit pipeline. Low cost, no LLM in the loop, unambiguous failures.
Not the priority right now.

### Spec Writer + Reviewer + Deduper + Verifier (5-agent pipeline)
Proposed: Five agent roles with strict separation of duties.
Rejected because: more moving parts = more failure modes. Reviewer and
Verifier are mostly prompt-level enforcement with weak mechanical gates.
The three-agent pipeline (Auditor → Test Writer → Fixer) has the strongest
mechanical enforcement and covers the review function via the test gate.

## Pipeline Architecture

### Queue Structure

```
pipeline/
  queue/
    audit-findings/          # Auditor output (new bug reports)
      {id}.md                # One file per finding, YAML frontmatter + body
    test-results/
      confirmed/             # Test compiles + fails (bug is real)
        {id}.md
      rejected/              # Test passes (false positive) or bad test
        {id}.md
      blocked/               # Agent couldn't write test, needs engine change
        {id}.md
    fix-results/
      ready-for-review/      # All tests pass, diff ready for human review
        {id}.md
      failed/                # Fixer couldn't make tests pass
        {id}.md
  prompts/
    auditor-shared.md        # Shared prompt for code audit agents
    log-miner-shared.md      # Shared prompt for log mining agents
    test-writer-shared.md    # Shared prompt for test writing agents
    fixer-shared.md          # Shared prompt for fixer agents
  scripts/
    run_auditors.py          # Launch audit agents
    run_log_miners.py        # Launch log mining agents
    run_test_writers.py      # Launch test writer agents
    run_fixers.py            # Launch fixer agents
    validate_test.sh         # Mechanical gate: compile + must fail
    validate_fix.sh          # Mechanical gate: all tests pass
```

### Queue Item Format

Markdown with YAML frontmatter. Human-readable and machine-parseable:

```markdown
---
id: finding-001
source: code-audit | log-mine
card: Fiend Hunter
engine_file: mtg-engine/src/triggers.rs
engine_line: 893
oracle_text: "When Fiend Hunter enters the battlefield, exile..."
created: 2026-04-12
---

## Bug Description

ETB trigger is suppressed when source leaves battlefield before resolution.

## Evidence

Oracle says: `When Fiend Hunter enters the battlefield, you may exile...`
Code does: `triggers.rs:893` checks `zone == Battlefield` at resolution.

## Affected Cards

- Fiend Hunter
- Armored Skaab
- Crossway Vampire
```

### Agent Specifications

#### 1. Code Auditor

Same as existing audit pipeline, but outputs to the queue instead of
`audits/reports/`.

| Property | Value |
|----------|-------|
| Input | Card name + pre-fetched Scryfall oracle text |
| Reads | Card impl, engine source, tests |
| Writes | `pipeline/queue/audit-findings/{card}.md` |
| Gate | Report has required sections, quotes are present |
| Model | claude-sonnet-4-6 (cost-effective for reading) |

#### 2. Log Miner

New agent type. Reads game logs and identifies rule violations.

| Property | Value |
|----------|-------|
| Input | Game log file (or section of it) + oracle text cache |
| Reads | Log file, engine source, oracle text |
| Writes | `pipeline/queue/audit-findings/{id}.md` |
| Gate | Must cite specific log line + oracle text |
| Model | claude-sonnet-4-6 |

#### 3. Test Writer

| Property | Value |
|----------|-------|
| Input | One finding from `pipeline/queue/audit-findings/` |
| Reads | Engine source, `tests/common/mod.rs`, existing tests, oracle text |
| Writes | `mtg-engine/tests/pipeline_bugs*.rs` (append only) |
| Gate | `cargo check --tests` passes AND `cargo test -- {test_name}` fails with assertion error |
| Anti-laziness | Banned phrases: TODO, FIXME, "further investigation", "would need to", "beyond the scope" |
| On pass (false positive) | Finding moved to `test-results/rejected/` |
| On compile fail | Finding moved to `test-results/rejected/` |
| On assertion fail | Finding moved to `test-results/confirmed/` with test path |
| On blocked | Finding moved to `test-results/blocked/` with required engine change description |
| Model | claude-sonnet-4-6 |

Mechanical validation (run by harness after agent completes):
```
1. grep for banned phrases -> reject if found
2. cargo check --tests -> reject if fails
3. cargo test -- {test_name} -> if passes, it's a false positive (reject)
4. check exit is assertion error, not panic -> reject if wrong failure type
5. count assert!/assert_eq! calls >= 1 -> reject if none
6. CONFIRMED
```

#### 4. Fixer

| Property | Value |
|----------|-------|
| Input | One confirmed finding from `test-results/confirmed/` |
| Reads | Engine source, the failing test (read-only) |
| Writes | `mtg-engine/src/**` only. NO test files. |
| Gate | `cargo test` all pass, `cargo check` zero warnings |
| Model | claude-sonnet-4-6 |

### Invocation

The user should be able to say:
- "launch 3 code audit agents on complex cards"
- "run test writers on the audit findings queue"
- "run a fixer on finding-001"

**Uses Claude Code subscription, NOT Anthropic API.** Agents are spawned
via `claude -p` (Claude Code CLI in print mode). This means:
- Uses the same subscription as interactive Claude Code
- No `ANTHROPIC_API_KEY` needed
- `--permission-mode auto` auto-approves all tool calls
- `--model opus` and `--effort max` for highest quality
- Agents have full tool access (bash, web, read, write, compile)
- Write restrictions are enforced by POST-VALIDATION, not tool restrictions
- The batch runner (`~/agent/claude-code/src/batch.ts`) is NOT used

### CLI Tool

`pipeline/cli.py` is the single entry point:
```bash
./pipeline/cli.py audit --cards "Olivia Voldaren,Fiend Hunter"
./pipeline/cli.py audit --complex 5
./pipeline/cli.py test
./pipeline/cli.py test --findings olivia-01
./pipeline/cli.py fix --finding olivia-01
./pipeline/cli.py status
```

It handles everything: prompt construction, agent spawning via `claude -p`,
result validation, metrics logging, and queue management. No manual
intervention needed.

Can also be invoked by Claude in a conversation via Bash tool:
```
Bash: python3 pipeline/cli.py audit --cards "Fiend Hunter" --model opus
```

### Concurrency Rules

- **Auditors**: Parallelize freely. One agent per card. Each writes to
  separate finding files (named `{date}-{card}-{NN}.md`). No conflicts
  because the orchestrator assigns different cards.
- **Test writers**: Parallelize, but each writes to its OWN test file
  (`pipeline_bugs_{finding_id}.rs`). Never share a test file.
- **Fixers**: Run ONE at a time, or in separate git worktrees. Concurrent
  fixers editing the same engine files will conflict. Worktree isolation
  via `Agent(isolation: "worktree")` handles this.

### Queue Ownership

Agents do NOT self-serve from queues. The orchestrating Claude (in the
main conversation) reads the queue, assigns specific work items to specific
agents, and moves files between queue directories based on results.

The user is the serialization point between phases:
- "Launch N audit agents on cards X, Y, Z" → orchestrator assigns 1 card
  per agent, agents write to `audit-findings/`
- "Run test writers on the queue" → orchestrator reads `audit-findings/`,
  assigns 1 finding per test-writer agent
- "Run fixer on finding-001" → orchestrator assigns 1 confirmed finding
  to fixer

In a fresh conversation, the new Claude reads queue directories to see
current state. Unprocessed items are whatever remains in `audit-findings/`.
Processed items have been moved to `test-results/{confirmed,rejected,blocked}/`.

### File Naming

Finding files: `{YYYY-MM-DD}-{card_snake_case}-{NN}.md`
Test files: `mtg-engine/tests/pipeline_bugs_{finding_id}.rs`

Date prefix prevents collisions when the same card is re-audited.

### One Agent Per Card (Auditors)

One auditor agent audits one card completely and produces all findings for
that card. Each finding is a separate file in the queue (for downstream
processing), but the agent does the full audit in one session. This matches
the existing audit pipeline design.

### Known Failure Modes & Mitigations

| Failure Mode | Mitigation |
|---|---|
| Auditor only reads card file, misses engine bugs | Required engine checks (8a-8f) force agents to trace zone cleanup, trigger dispatch, ability offering, subtype checks, damage path, and effect duration in the engine. Data: 5 identical Olivia audits — the 1 run that read state.rs::move_object found a second bug the other 4 missed. |
| Auditor hallucinates oracle text | Oracle pre-fetched from Scryfall, agent can't use training data |
| Auditor claims mismatch without evidence | Must quote both sides exactly |
| Test writer says "can't test without engine changes" | Must produce structured BLOCKED report with file:line. Or express bug as observable game state (life, zones, P/T) |
| Test writer is lazy / defers work | Banned phrases auto-rejected. Must compile AND fail gate. |
| Test fails for wrong reason | Human review of test writer output |
| Fixer weakens test to make it pass | No write access to test files |
| Fixer introduces regression | Full `cargo test` suite must pass |
| Fixer makes hacky fix | Human review of diff |
| Same engine bug reported N times | Acceptable — N regression tests for one root cause. Lightweight dedup via grep on engine file:line. |
| Fix reveals masked bugs | Expected. Track root causes fixed / total. |
| UI/harness bugs can't be unit-tested | Separate from engine pipeline. Handle manually or with snapshot tests. |

## Implementation Status

- [x] Pipeline directory structure (`pipeline/queue/`, `pipeline/prompts/`, etc.)
- [x] Queue item schema (YAML frontmatter + markdown body)
- [x] Auditor shared prompt (`pipeline/prompts/auditor.md`)
- [ ] Log miner shared prompt
- [x] Test writer shared prompt (`pipeline/prompts/test-writer.md`)
- [x] Fixer shared prompt (`pipeline/prompts/fixer.md`)
- [x] `validate_test.sh` gate script
- [x] `validate_fix.sh` gate script
- [x] Metrics system (`pipeline/metrics/`, `pipeline/scripts/metrics.py`)
- [x] First test run (Olivia Voldaren audit — 1 finding, correct format)
- [x] 5x repeat audit experiment — all 5 found same bug, 1 found a second
- [x] Required engine checks (8a-8h) added to prompt — rerun found 4 bugs vs 1
- [x] Merged fix-audit-bugs-LCCuS branch — fixed 50+ previously-failing tests
- [x] Extracted insights from fixed bugs → added 8f (hexproof/targeting), 8g (token/copy), 8i (rulings coverage)
- [ ] Test Writer end-to-end test
- [ ] Fixer end-to-end test
- [ ] Log miner prompt + test
- [ ] Multi-agent batch run test

## Conversation History (for context resumption)

### Arc of the design conversation (2026-04-10 through 2026-04-12)

1. User asked about model-based testing (article about D&D). I explained the
   concept and assessed applicability to MTG.

2. Discussed fuzzing: random-legal-move fuzzer with invariants. Agreed on
   Tier 1 (structural) + Tier 3 (legal_actions applyability) as most valuable.
   Deferred implementation.

3. Discussed spec-based testing. Started with YAML specs + runner, then
   Rust DSL, then plain Rust tests. Each iteration simplified.
   User pushed back at each layer of abstraction ("why do you need a DSL?",
   "why not just standard Rust tests?"). Final answer: no new abstractions,
   just tests using existing helpers.

4. User shared frustration: previous agent pipeline (auditor → verifier →
   fixer) didn't converge. "Sometimes agents don't listen. There are still
   a ton of bugs allegedly."

5. I explored the existing audit infrastructure (`audits/`, `AUDIT_BUGS.md`,
   `run_audit.py`). Found the pipeline is actually well-designed — oracle
   pre-fetched, quote-both-sides rule, structured reports. The issue is
   fix throughput, not find quality.

6. Diagnosed the convergence problem: not broken pipeline, but (a) bugs
   not deduped by root cause, (b) fix throughput bottleneck, (c) masked
   bug cascades, (d) no gate between finding and fixing.

7. Designed three-agent pipeline: Auditor → Test Writer → Fixer.
   Test writer's "must fail" gate replaces the reviewer.
   Mechanical enforcement at each step.

8. Addressed specific failure modes from user's experience:
   - "Can't write test without engine changes" → require BLOCKED report
     with specific file:line, or express bug as observable state change.
   - "Agents are lazy" → banned phrases, minimum assertions, compilation
     gate, "must compile AND fail" requirement.

9. User requirements for implementation:
   - Queue-based, committed to git
   - Phases run independently
   - Uses Claude Code subscription (batch runner), not Anthropic API
   - User can invoke via natural language ("launch 3 audit agents on X")
   - Mechanical harness with file editing constraints

### User preferences (from memory + conversation)

- Small incremental commits
- Code must work for the right reasons, not just produce correct output
- Engine limitations that cause wrong card behavior must be flagged
- NEVER simplify cards without telling the user first
- Correctness over convenience
- Skeptical of agent reliability — wants mechanical enforcement
- Values honesty about what works and what doesn't
