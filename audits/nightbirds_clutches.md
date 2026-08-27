## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/154/nightbirds-clutches?utm_source=api
**Type line**: `Sorcery` — {1}{R}
**Oracle text**:
```
Up to two target creatures can't block this turn.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Up to two** target creatures can't block this turn" — a blocking
  restriction until end of turn, not a tap: PASS
- It applies whether or not the creature is untapped, unlike tapping it: PASS
- Flashback {3}{R}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The block restriction and the flashback: `cards_flashback.rs`, `combat_rules.rs`
