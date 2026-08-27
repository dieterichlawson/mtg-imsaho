## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/24/moment-of-heroism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- +2/+2 and lifelink until end of turn, both as `TemporaryEffect`s so they
  expire together: PASS
- Ruling: "Multiple instances of lifelink on the same creature are redundant":
  PASS
- Lifelink applies to all damage the creature deals, not only combat damage: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the lifelink: `cards_pump_spells.rs`, `keywords_lifelink.rs`
