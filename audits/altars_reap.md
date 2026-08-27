## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/86/altars-reap?utm_source=api
**Type line**: `Instant` — {1}{B}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
```
**Status**: PASS

### Code issues
No issues found.

Draws two. The sacrifice is an additional cost paid at cast time (CR 601.2f), so it happens even if the spell is later countered — correctly not part of resolution.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
