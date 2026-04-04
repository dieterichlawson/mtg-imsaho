# Bug Verification Test Progress

Writing failing tests to verify each audit issue.

## Categories to test (behavioral bugs)
1. engine missing feature (20 issues)
2. subtype check misses tokens (18 issues)
3. auto-selects instead of player choice (15 issues)
4. engine: trigger dispatch/zone (11 issues)
5. engine: simultaneous events (9 issues)
6. engine: summoning sickness (4 issues)
7. "as long as" snapshot (4 issues)
8. engine: force-attack missing checks (3 issues)
9. engine: planeswalker damage (3 issues)
10. engine: protection targeting (2 issues)
11. "may" not optional (1 issue)
12. missing shuffle (1 issue)

## Categories to skip (cosmetic/non-testable)
- oracle text mismatch (5 issues)
- log message (13 issues)
- LLM knowledge (2 issues)
- test enshrines wrong behavior (4 issues)

## Progress

### TESTED (failing test written and confirmed)
1. **summoning sickness** — `bug_summoning_sickness_not_enforced_for_tap_abilities` (covers 4 issues across 3 cards)
2. **subtype check misses tokens** — `bug_victim_of_night_can_target_vampire_token` (covers 18 issues across 18 cards)
3. **ETB trigger zone check** — `bug_etb_trigger_suppressed_when_source_leaves` (covers 11 issues across 11 cards)

### TODO (next)
4. auto-selects instead of player choice (15 issues) — test Falkenrath Noble auto-targeting
5. "as long as" snapshot (4 issues) — need to fix Bonds of Faith test setup
6. engine: simultaneous events (9 issues) — Falkenrath Noble simultaneous death
7. engine: force-attack missing checks (3 issues) — Bloodcrazed Neonate + Pacifism
8. engine: planeswalker damage (3 issues)
9. engine: protection targeting (2 issues)
10. "may" not optional (1 issue)
11. missing shuffle (1 issue)

### SKIPPED (cosmetic, not testable as failing test)
- oracle text mismatch (5 issues)
- log message (13 issues)
- LLM knowledge (2 issues)
- test enshrines wrong behavior (4 issues)

### Test file
mtg-engine/tests/audit_bugs.rs
