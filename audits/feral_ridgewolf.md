## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/142/feral-ridgewolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 1/2
**Oracle text**:
```
Trample
{1}{R}: This creature gets +2/+0 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- No activation limit, so it stacks with itself: PASS
- Trample is printed, not granted by the ability: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Stacking activations: `activated_abilities.rs:an_unrestricted_pump_ability_stacks_with_itself`
