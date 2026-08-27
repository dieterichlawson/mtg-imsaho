## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/106/manor-skeleton?utm_source=api
**Type line**: `Creature — Skeleton` — {1}{B}, 1/1
**Oracle text**:
```
Haste
{1}{B}: Regenerate this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{B}: Regenerate this creature" — a regeneration shield, which taps the
  creature, removes its damage and removes it from combat when it applies
  (CR 701.15): PASS
- Haste, so it can attack the turn it arrives: PASS
- Shields stack, so two activations survive two lethal events: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Regeneration and haste: `cards_morbid_and_ltb.rs`, `activated_abilities.rs`
