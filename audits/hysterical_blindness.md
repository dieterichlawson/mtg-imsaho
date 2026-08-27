## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/59/hysterical-blindness?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```
**Status**: PASS

### Code issues
No issues found.

- "Creatures your opponents control get -4/-0 until end of turn" — snapshots the
  affected creatures at resolution and pushes one per-target `ModifyPT`, which is
  what CR 611.2c requires of a spell. This card had it right where four others in
  the set did not.
- -4/-0, not -4/-4: power only.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
