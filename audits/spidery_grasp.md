## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/204/spidery-grasp?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Spidery Grasp can target a creature that's already untapped. It will
  still get +2/+4 and gain reach" — the untap is not a condition: PASS
- Untapping an attacking creature does not remove it from combat: PASS
- Reach until end of turn lets it block a flier this turn only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The untap, pump and reach: `cards_pump_spells.rs`, `evasion.rs`
