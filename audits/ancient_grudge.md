## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/127/ancient-grudge?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

- "Destroy target artifact", with a flashback cost in a different colour
  ({G} against a {1}{R} face). The flashback cost was verified exact set-wide,
  and `flashback.rs` covers casting from the graveyard and the exile afterwards
  (CR 702.33a).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
