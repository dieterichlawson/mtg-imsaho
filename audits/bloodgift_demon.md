## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/89/bloodgift-demon?utm_source=api
**Type line**: `Creature — Demon` — {3}{B}{B}, 5/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "At the beginning of **your** upkeep" — `TriggerScope::Your`, so it does not
  fire on the opponent's turn: PASS
- "**target player** draws a card and loses 1 life" — targeted, so it can point
  at yourself for the draw or at an opponent for the life: PASS
- Life **loss**, not damage, through `lose_life`: PASS
- CR 113.7a: killing the Demon in response does not counter the trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep scope, the target and the life loss: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
