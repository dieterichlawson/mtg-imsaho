## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/21/mentor-of-the-meek?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**another** creature **you control** with power 2 or less" — all three
  conditions, and the power is read as the creature *enters* (CR 603.2) rather
  than at resolution, so a creature pumped in response still drew the card: PASS
- "you **may** pay {1}. If you do, draw a card" — an optional cost, declined
  without drawing: PASS
- `effective_power` rather than printed power, so a token or a buffed creature
  is judged by what it actually is: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power threshold at entry time: `trigger_snapshots.rs`, `cards_complex_creatures.rs`
