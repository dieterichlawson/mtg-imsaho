## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/173/caravan-vigil?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Search your library for a basic land card ... put it into your hand, then
  shuffle" — a Basic supertype *and* the Land card type, so a nonbasic is not
  offered: PASS
- Every basic in the library is offered, not the first found: PASS
- "Morbid — **You may** put that card onto the battlefield **instead** ... if a
  creature died this turn" — the choice is offered only when the condition
  holds, and declining puts it in hand: PASS
- The morbid condition is checked at resolution: PASS
- Onto the battlefield untapped, and it does not count as a land drop: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The search, the morbid choice, and declining: `auto_pick.rs`, `cards_morbid_and_ltb.rs`
