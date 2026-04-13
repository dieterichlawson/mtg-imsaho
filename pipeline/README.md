# Bug Pipeline

A ticket-based pipeline for finding and fixing bugs in the MTG engine.
Each ticket tracks one bug through its lifecycle. Agents write to staging;
Python owns ticket state and frontmatter.

## Architecture

```
Auditor → staging/ → Python creates tickets → Test Writer → Fixer → Human
```

## Ticket Lifecycle

```
new → confirmed → fixed → merged
   ↘ rejected (terminal)
   ↘ blocked (manual intervention)
              ↘ failed (can retry)
```

## Directory Structure

```
pipeline/
  cli.py              # main entry point
  staging/             # ephemeral agent output (cleaned after processing)
  tickets/             # permanent ticket files ({id}.md)
  prompts/             # agent prompts
  scripts/             # validation scripts, metrics
  metrics/             # JSONL tracking files
```

## How to Run

```bash
# Audit cards
./pipeline/cli.py audit --cards "Olivia Voldaren,Fiend Hunter"
./pipeline/cli.py audit --cards "Olivia Voldaren" --parallelism 5

# Run test writers on new tickets
./pipeline/cli.py test
./pipeline/cli.py test --tickets olivia_voldaren-01,olivia_voldaren-02

# Fix a confirmed ticket
./pipeline/cli.py fix --ticket olivia_voldaren-01

# List tickets
./pipeline/cli.py tickets
./pipeline/cli.py tickets --status new
./pipeline/cli.py tickets --card "Olivia"

# Show full ticket history
./pipeline/cli.py show olivia_voldaren-01

# Accept a fix (human gate)
./pipeline/cli.py accept olivia_voldaren-01

# Metrics dashboard
./pipeline/cli.py status

# Group tickets by engine root cause
./pipeline/cli.py dedup

# Dry run any command
./pipeline/cli.py audit --cards "Fiend Hunter" --dry-run
```

## Mechanical Enforcement

| Agent | Can Write | Gate |
|-------|-----------|------|
| Auditor | `pipeline/queue/audit-findings/` | Report format, quotes present |
| Test Writer | `mtg-engine/tests/pipeline_bugs*.rs` (append) | Compiles + fails + no banned phrases + >= 1 assertion |
| Fixer | `mtg-engine/src/**` only | All tests pass + zero warnings + no test files modified |

## Validation Scripts

```bash
# After test writer completes:
./pipeline/scripts/validate_test.sh mtg-engine/tests/pipeline_bugs.rs test_name

# After fixer completes:
./pipeline/scripts/validate_fix.sh test_name
```

## Metrics

All metrics are tracked in `pipeline/metrics/`:
- `runs.jsonl` — one entry per agent invocation (model, role, result)
- `findings.jsonl` — one entry per finding state transition (lifecycle)
- `schema.md` — field definitions and derivable metrics

View the dashboard:
```bash
python3 pipeline/scripts/metrics.py              # Full dashboard
python3 pipeline/scripts/metrics.py --funnel     # Just the funnel
python3 pipeline/scripts/metrics.py --agents     # Agent performance by model
python3 pipeline/scripts/metrics.py --trends     # Trends over time
python3 pipeline/scripts/metrics.py --cards      # Per-card breakdown
python3 pipeline/scripts/metrics.py --json       # Machine-readable
```

Key metrics:
- **False positive rate**: test_rejected / (confirmed + rejected) — are auditors finding real bugs?
- **Fix success rate**: fix_succeeded / attempted — can fixers handle the bugs?
- **Clean card rate**: cards with 0 findings / total audited — are we making progress?
- **Finding velocity**: findings per audit run — trending down = progress
- **Backlog**: confirmed findings not yet fixed

The orchestrating Claude logs metrics after each agent run. Agents do NOT
write metrics — the orchestrator does, based on validated results.

## Adding to the Queue Manually

Create a file in `pipeline/queue/audit-findings/` with this format:

```markdown
---
id: manual-001
source: manual
card: Card Name
card_file: mtg-engine/src/cards/isd/card_name.rs
created: 2026-04-12
---

## Bug Description

Description of the bug.

## Evidence

**Oracle text says:**
> exact quote

**Code does:**
> exact quote or description with file:line

## Engine Path

Which engine files are involved.

## Affected Cards

- Card Name
```
