## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/96/dead-weight?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -2/-2.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- -2/-2 kills a 2/2 by state-based action rather than by destruction, so
  indestructible does not save it (CR 704.5f): PASS
- The Aura goes to the graveyard with the creature it killed (CR 704.5m): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The debuff and the SBA death: `enchantments.rs`, `state_based_actions.rs`
