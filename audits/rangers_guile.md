## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/m21/199/rangers-guile?utm_source=api
**Type line**: `Instant` — {G}
**Oracle text**:
```
Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target creature **you control**" — `TargetFilter::YouControl` and the card's
  own `is_valid_target`: PASS
- Hexproof until end of turn is what makes a targeted removal spell already on
  the stack fizzle (CR 608.2b) — the interaction the ability exists for: PASS
- Granting hexproof to your own creature does not stop *your* spells targeting
  it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the hexproof save: `cards_pump_spells.rs`, `fizzle.rs:a_target_that_gained_hexproof_in_response_is_skipped_and_the_rest_resolve`
