## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/187/hamlet-captain?utm_source=api
**Type line**: `Creature — Human Warrior` — {1}{G}, 2/2
**Oracle text**:
```
Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever this creature attacks **or blocks**" — two declared triggers and two
  hooks, so both directions fire: PASS
- "**other** Humans you control" — the Captain excludes itself: PASS
- The set of Humans is fixed when the trigger resolves (CR 611.2c), so one
  arriving later gets nothing: PASS
- `has_subtype` reads granted subtypes, so a Human token counts: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both triggers and the self-exclusion: `combat_rules.rs`, `subtype.rs`
