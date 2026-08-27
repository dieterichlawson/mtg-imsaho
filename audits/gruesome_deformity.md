## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/103/gruesome-deformity?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Intimidate is the printed keyword — "can't be blocked except by artifact
  creatures and/or creatures that share a color with it" — and not menace: PASS
- The evasion is evaluated against the blocker's colours at declare-blockers: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Intimidate blocking: `evasion.rs`, `enchantments.rs`
