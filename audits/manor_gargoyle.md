## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/228/manor-gargoyle?utm_source=api
**Type line**: `Artifact Creature — Gargoyle` — {5}, 4/4
**Oracle text**:
```
Defender
This creature has indestructible as long as it has defender.
{1}: Until end of turn, this creature loses defender and gains flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}: This creature loses defender and gains flying until end of turn" — both
  halves, and losing defender is a keyword removal rather than a P/T change:
  PASS
- Indestructible is printed and is not affected by the ability: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Losing defender and gaining flying: `cards_activated_abilities.rs:manor_gargoyle_loses_defender_and_gains_flying`
