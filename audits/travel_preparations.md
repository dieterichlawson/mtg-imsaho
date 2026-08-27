## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/206/travel-preparations?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Put a +1/+1 counter on each of up to two target creatures.
Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**up to two** target creatures" — `UpToTargets(2, Creature)`, so it is
  castable with zero, one or two targets: PASS
- "each of" — one counter on each, not two on one: PASS
- Flashback {1}{W} is a different colour from the front cost {1}{G}, and the
  card is exiled after the flashback resolution (CR 702.33a): PASS
- One of two targets becoming illegal leaves the other still getting its counter
  (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Up-to targeting and the counters: `cards_flashback.rs`, `multi_target_and_mill.rs`
- Flashback exile: `cards_flashback.rs`
