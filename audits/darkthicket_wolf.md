## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/175/darkthicket-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {1}{G}, 2/2
**Oracle text**:
```
{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Activate only once each turn" — `once_per_turn: true`, tracked per object and
  reset at untap: PASS
- The pump is until end of turn, not permanent: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- One activation pumps: `activated_abilities.rs:a_pump_ability_changes_the_creature_it_is_activated_on`
- The once-per-turn restriction: `activated_abilities.rs`
