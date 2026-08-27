## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/143/furor-of-the-bitten?utm_source=api
**Type line**: `Enchantment — Aura` — {R}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If the enchanted creature can't attack for any reason (such as being
  tapped or having come under that player's control that turn), then it doesn't
  attack." An attack requirement cannot force an illegal attack (CR 508.1d):
  PASS
- "attacks each combat if able" is a requirement on the creature, so it follows
  the Aura rather than the controller: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the attack requirement: `enchantments.rs`, `combat_requirements.rs`
