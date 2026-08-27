## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/33/smite-the-monstrous?utm_source=api
**Type line**: `Instant` — {3}{W}
**Oracle text**:
```
Destroy target creature with power 4 or greater.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power **4 or greater**" read through `effective_power`, so a pumped 2/2 is a
  legal target and a debuffed 5/5 is not: PASS
- The check runs again on resolution: shrinking the creature in response makes
  it fizzle (CR 608.2b): PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power threshold at cast and at resolution: `cards_removal.rs`, `resolution_time_checks.rs:a_target_that_stops_qualifying_makes_the_spell_fizzle`
