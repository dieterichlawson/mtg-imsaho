## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/234/travelers-amulet?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Search your library for a basic land card" — a Basic supertype *and* the Land
  card type, so a nonbasic land is not offered: PASS
- Every basic in the library is offered, not the first one found — a B/R deck
  splashing green must be able to tutor the Forest specifically: PASS
- "...then shuffle": PASS
- Sacrificing the Amulet is a cost, paid on activation: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Every basic offered: `auto_pick.rs:travelers_amulet_offers_every_basic_land_in_the_library`
- The library is shuffled after the search: `auto_pick.rs:bug_bf_travelers_amulet_shuffles_library_after_search`
- The basic reaches hand: `cards_equipment_and_artifacts.rs:travelers_amulet_finds_basic_land`
