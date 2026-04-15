# Dedup Agent — Shared Prompt

You are writing a consolidation input file that merges several existing
bug tickets into one "merged" ticket. The user has identified a cluster
of tickets that appear to share a single engine root cause.

## Your Task

You will receive N ticket bodies in the per-agent prompt. Write a single
markdown file that consolidates them into one ticket-with-N-tests, ready
for `cli.py consolidate` to ingest.

## Rules

1. **One engine root cause.** The engine fix must be the SAME across all
   tickets you include. If you think any ticket is not a true member of
   the cluster (different root cause, unrelated bug sharing superficial
   wording), SAY SO in a `## Excluded` section at the end and do not
   include it in Tests.
2. **One test per included ticket.** Strict one-to-one. The
   consolidator Python will fail if two tests share a `Source ticket:`.
3. **Test slugs are snake_case**, unique within the file, and
   distinctive enough to identify the scenario — not just the card
   name.
4. **Scenarios are executable.** Each scenario is a concrete
   setup + action + assertion. "Verify the bug" is not a scenario;
   "Cast Olivia's ability targeting a protection-from-black creature,
   verify no damage dealt and no Vampire subtype added" is.
5. **Description explains THE BUG once,** not N summaries. State the
   engine root cause in one coherent paragraph, with CR references.
   Readers should leave understanding what the single fix needs to do.
6. **Engine path lists the fix location(s),** not per-card paths. If
   the bug lives in `engine.rs:3424-3478`, list that. Do not list
   `olivia_voldaren.rs:104` — that's a symptom location.

## What good consolidation looks like

- Description reads like a single bug report, not a table of contents
- Engine path points at the ONE (or a small, related set of) place(s)
  where the fix lands
- Each test scenario is observable game state (life totals, zone
  contents, P/T, counter counts, whether a trigger is on the stack),
  not engine internals
- Slugs describe what is being asserted, e.g.
  `test_angel_trigger_resolves_after_source_death`,
  not `test_angel_01`

## Common failure modes to avoid

- **Mixing two root causes into one consolidation** (e.g. "these are
  all zone-change bugs" when some are about subtypes and others about
  `TemporaryEffect`s). Split them; emit `## Excluded` for the outliers.
- **Scenarios that just restate the description.** A scenario must be
  directly implementable as a test.
- **Slug collisions** across tests. Each slug must be unique.
- **Forgetting `Source ticket:`** on a test. Python requires it.
- **Treating "same engine file" as the same bug.** `engine.rs` has
  thousands of lines; two bugs in `triggers.rs` can have independent
  root causes.

## Output Format

Write to the staging path specified in your per-agent prompt. This EXACT
format (Python parses it):

```markdown
---
slug: {short-kebab-case-slug}
---

# {Title — one line summarizing the engine bug, ideally with CR reference}

## Description
{One or two paragraphs explaining THE engine root cause. Do NOT summarize
each ticket. Explain the bug and reference the relevant CR.}

## Engine path
- {file:line} ({what this location does})
- {more file:line entries if the fix spans a small related set}

## Tests

### {test_slug_1}
Source ticket: {ticket-id}
Scenario: {Specific setup, action, and assertion.}

### {test_slug_2}
Source ticket: {other-ticket-id}
Scenario: ...
```

If you excluded any tickets from the cluster, append:

```markdown
## Excluded

- {ticket-id}: {why this ticket does not belong in this consolidation}
```

Do not include any other sections. Do not write frontmatter beyond
`slug:`. Python handles the rest of the ticket metadata.
