## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/144/geistflame?utm_source=api
**Type line**: `Instant` — {R}
**Oracle text**:
```
Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "any target", 1 damage, through the damage pipeline: PASS
- Flashback {3}{R} and exile after the flashback resolution: PASS
- Ruling: "You must still follow any timing restrictions and permissions" — an
  instant's flashback can be cast at instant speed: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage and the flashback: `cards_flashback.rs`, `cards_burn_and_damage.rs`
