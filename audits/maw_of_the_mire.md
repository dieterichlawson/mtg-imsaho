## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/108/maw-of-the-mire?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Destroy target land. You gain 4 life.
```

**Status**: ISSUE

### Code issues
See below.


- The life gain was written out by hand rather than going through
  `GameState::change_life`. Collapsed in the set-wide sweep; see the guard
  `test_suite_guards.rs::only_change_life_writes_a_life_total`.

### Tricky interactions checked
- Ruling: "If the targeted land is an illegal target by the time Maw of the Mire
  resolves, it won't resolve and none of its effects will occur. **You won't
  gain 4 life.**" The life gain is gated on the target still being on the
  battlefield — *not* on the destroy succeeding, so an indestructible land
  survives and you still gain 4, which is what "the spell resolved" means:
  PASS
- `try_destroy`, so indestructible and regeneration apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The life gain gated on target legality: `cards_removal.rs`
