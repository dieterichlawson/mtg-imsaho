# Pipeline Metrics Schema

Two append-only JSONL files, committed to git. The orchestrating Claude
appends entries after each agent run and validation step. Agents themselves
do NOT write metrics — the orchestrator does, based on validated results.

## `runs.jsonl` — One entry per agent invocation

```json
{
  "run_id": "2026-04-12-olivia-audit-01",
  "timestamp": "2026-04-12T17:30:00Z",
  "role": "auditor | test-writer | fixer",
  "model": "claude-sonnet-4-6",
  "card": "Olivia Voldaren",
  "finding_id": null,
  "findings_created": 3,
  "test_result": null,
  "fix_result": null,
  "validation_passed": true,
  "rejection_reason": null,
  "notes": ""
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `run_id` | string | Unique ID: `{date}-{card/finding}-{role}-{NN}` |
| `timestamp` | ISO 8601 | When the agent completed |
| `role` | enum | `auditor`, `test-writer`, `fixer`, `log-miner` |
| `model` | string | Model used (e.g., `claude-sonnet-4-6`) |
| `card` | string | Card name (for auditors) |
| `finding_id` | string? | Finding being worked on (test-writers, fixers) |
| `findings_created` | int | Number of findings produced (auditors) |
| `test_result` | enum? | `confirmed`, `rejected`, `blocked` (test-writers) |
| `fix_result` | enum? | `fixed`, `failed` (fixers) |
| `validation_passed` | bool | Did the mechanical gate pass? |
| `rejection_reason` | string? | Why validation failed (if it did) |
| `total_tokens` | int? | Total tokens consumed by the agent |
| `tool_uses` | int? | Number of tool calls the agent made |
| `duration_seconds` | int? | Wall-clock time in seconds |
| `notes` | string | Free-form notes |

## `findings.jsonl` — One entry per finding state transition

```json
{
  "finding_id": "olivia-01",
  "timestamp": "2026-04-12T17:30:00Z",
  "event": "created",
  "card": "Olivia Voldaren",
  "source": "code-audit",
  "engine_file": "mtg-engine/src/triggers.rs",
  "description": "First ability damage bypasses central damage helper",
  "run_id": "2026-04-12-olivia-audit-01"
}
```

### Events (finding lifecycle)

```
created → test_confirmed → fix_succeeded → merged
       ↘ test_rejected
       ↘ test_blocked
                         ↘ fix_failed
```

| Event | Meaning |
|-------|---------|
| `created` | Finding written to `audit-findings/` by auditor or log miner |
| `test_confirmed` | Test writer produced a failing test. Finding is real. |
| `test_rejected` | Test passes or can't be written. False positive or untestable. |
| `test_blocked` | Test writer needs engine change to write the test. |
| `fix_succeeded` | Fixer made the test pass. All tests green. |
| `fix_failed` | Fixer couldn't make it work. |
| `merged` | Human reviewed and merged the fix. |

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `finding_id` | string | Matches the `id` in the finding's YAML frontmatter |
| `timestamp` | ISO 8601 | When this event occurred |
| `event` | enum | See lifecycle above |
| `card` | string | Card name |
| `source` | enum | `code-audit`, `log-mine`, `manual` |
| `engine_file` | string? | Primary engine file involved |
| `description` | string | One-line bug summary |
| `run_id` | string | Which agent run produced this event |
| `test_name` | string? | Rust test function name (after test_confirmed) |
| `test_file` | string? | Rust test file path (after test_confirmed) |
| `files_changed` | string[]? | Engine files modified (after fix_succeeded) |

## Key Metrics Derivable

### Agent Quality
- **False positive rate**: `test_rejected / (test_confirmed + test_rejected)` per model
- **Blocked rate**: `test_blocked / total_tested` — how often can't test
- **Fix success rate**: `fix_succeeded / (fix_succeeded + fix_failed)` per model
- **Compile rate**: test-writer validation_passed rate

### Progress
- **Funnel**: created → confirmed → fixed → merged (counts at each stage)
- **Finding velocity**: findings created per audit run (trending down = progress)
- **Fix velocity**: fixes merged per day/week
- **Clean card rate**: cards audited with 0 findings / total audited
- **Backlog**: confirmed findings not yet fixed

### Root Cause Analysis
- **Engine hotspots**: engine_file frequency in findings
- **Card hotspots**: cards with most findings
- **Dedup ratio**: unique engine_files / total findings

### Trends (time series)
- All of the above over time, per date or per run batch
