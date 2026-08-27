## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/151/kessig-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 3/1
**Oracle text**:
```
{1}{R}: This creature gains first strike until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{R}: This creature gains first strike until end of turn" — no activation
  limit, so it stacks harmlessly with itself: PASS
- Granted mid-combat after first-strike damage has been dealt does not give it a
  second damage step (CR 510.4): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The keyword grant: `activated_abilities.rs:a_pump_ability_changes_the_creature_it_is_activated_on`
