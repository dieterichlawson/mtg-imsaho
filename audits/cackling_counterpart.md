## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/46/cackling-counterpart?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "a token that's a **copy of** target creature you control" — copies the
  printed characteristics, not counters, Auras, or non-copy effects (CR 707.2):
  PASS
- "target creature **you control**": PASS
- The token is a token, so it ceases to exist if it leaves the battlefield: PASS
- Flashback {5}{U}{U}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The token copy and the flashback: `cards_flashback.rs`, `cards_complex_creatures.rs`
