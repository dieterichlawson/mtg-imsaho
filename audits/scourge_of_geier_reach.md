## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/162/scourge-of-geier-reach?utm_source=api
**Type line**: `Creature — Elemental` — {3}{R}{R}, 3/3
**Oracle text**:
```
This creature gets +1/+1 for each creature your opponents control.
```
**Status**: PASS

### Code issues
No issues found.

- "gets +1/+1 for each creature your opponents control" — `dynamic_pt` returns
  `3 + N`, and since `dynamic_pt` supplies the *base* in `effective_power`,
  counters and anthems still layer on top correctly.
- Recomputed on every read, which is right for a characteristic-defining
  ability: the bonus tracks the opponent's board as it changes.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
