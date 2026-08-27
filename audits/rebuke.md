## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/29/rebuke?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Destroy target attacking creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **attacking** creature" — read from combat state, so it is only
  castable once attackers are declared: PASS
- CR 506.4: a creature removed from combat stops being an attacking creature, so
  the spell fizzles if that happens in response: PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Attacking-only targeting and the removal-from-combat fizzle: `cards_removal.rs`, `combat_rules.rs`
