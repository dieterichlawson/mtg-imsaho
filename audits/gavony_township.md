## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/239/gavony-township?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "+1/+1 **counter** on each creature you control" — counters, not a continuous
  effect, so CR 611.2c's snapshot rule does not apply and they persist past end
  of turn: PASS
- "each creature **you control**" — no targeting, so it cannot be responded to
  by making a creature untargetable: PASS
- The set of creatures is read when the ability resolves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Counters on each creature: `cards_activated_abilities.rs`
- The {T} cost's legality: `tap_cost_legality.rs`
