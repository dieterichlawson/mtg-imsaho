## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/40/urgent-exorcism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Destroy target Spirit or enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **Spirit** or **enchantment**" — a subtype on one side and a card type
  on the other, so a Spirit creature and a non-Spirit Aura are both legal: PASS
- `has_subtype` reads the object's granted subtypes as well as the printed ones,
  so a token Spirit qualifies: PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both halves of the filter: `cards_removal.rs`, `subtype.rs`
