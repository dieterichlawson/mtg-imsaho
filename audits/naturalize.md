## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/197/naturalize?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Destroy target artifact or enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "artifact **or** enchantment" — both, and an artifact creature qualifies: PASS
- `try_destroy`, so indestructible survives: PASS
- The target is re-checked on resolution, so a permanent that stopped being an
  artifact or enchantment makes it fizzle (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Destroying each type, and indestructible: `cards_removal.rs`, `fizzle.rs`
