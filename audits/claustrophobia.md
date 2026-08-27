## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/48/claustrophobia?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{U}{U}
**Oracle text**:
```
Enchant creature
When this Aura enters, tap enchanted creature.
Enchanted creature doesn't untap during its controller's untap step.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this Aura enters, **tap** enchanted creature" is a separate ETB trigger,
  and "doesn't untap during its controller's untap step" is the static half —
  two abilities, not one: PASS
- The ETB trigger taps whatever it is attached to at resolution, so removing the
  Aura in response leaves the creature untapped: PASS
- `PreventUntap` is scoped `Attached`, so it stops when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The ETB tap and the untap prevention: `cards_auras.rs`, `enchantments.rs`
