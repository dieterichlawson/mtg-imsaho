## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/12/elder-cathar?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "put a +1/+1 counter on **target creature you control**" — targeted at
  CR 603.3d time, so it is chosen when the death trigger goes on the stack: PASS
- "**If that creature is a Human**, put two +1/+1 counters on it **instead**" —
  two, not one plus one, and the check runs at resolution so a creature that
  became a Human in between gets two: PASS
- `has_subtype` covers granted and token subtypes: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both counter amounts: `cards_morbid_and_ltb.rs`, `subtype.rs`
