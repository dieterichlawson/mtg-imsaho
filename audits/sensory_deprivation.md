## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/74/sensory-deprivation?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -3/-0.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- -3/-0 only, so a 2/2 becomes 0/2 and survives — toughness is untouched: PASS
- Negative power deals no combat damage rather than negative damage: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power reduction: `enchantments.rs`
