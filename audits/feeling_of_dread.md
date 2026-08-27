## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/14/feeling-of-dread?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Up to two** target creatures" — castable with zero, one or two: PASS
- Tapping an attacking creature does not remove it from combat (CR 506.4c), so
  this is a blocker-remover rather than a combat trick: PASS
- One of two targets becoming illegal leaves the other still tapped: PASS
- Flashback {1}{U} is a different colour from the {1}{W} front cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Up-to targeting and the flashback: `cards_flashback.rs`, `combat_rules.rs`
