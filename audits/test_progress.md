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
- Current: starting
- Test file: mtg-engine/tests/audit_bugs.rs
