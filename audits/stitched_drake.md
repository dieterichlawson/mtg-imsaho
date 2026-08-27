## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/80/stitched-drake?utm_source=api
**Type line**: `Creature — Zombie Drake` — {1}{U}{U}, 3/4
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve`; removed.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
