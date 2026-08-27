## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/170/avacyns-pilgrim?utm_source=api
**Type line**: `Creature — Human Monk` — {G}, 1/1
**Oracle text**:
```
{T}: Add {W}.
```
**Status**: PASS

### Code issues
No issues found.

A single free `{T}: Add {W}` mana ability. The tap-cost conditions — battlefield, untapped, summoning sickness with the haste exception (CR 302.6) — are the engine's, applied centrally, and covered by `tap_cost_legality.rs`.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/170/avacyns-pilgrim?utm_source=api
**Type line**: `Creature — Human Monk` — {G}, 1/1
**Oracle text**:
```
{T}: Add {W}.
```

**Status**: PASS

### Code issues
No issues found.

"{T}: Add {W}" declared as a `ManaAbilityDef` with `requires_tap: true`,
`cost: ManaCost::free()`, `has_side_effects: false` — so it is a mana ability
(CR 605.1a): it does not use the stack and cannot be responded to. Human Monk,
both subtypes present.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_lands_and_mana_sources.rs` — taps for {W}; `mana_abilities.rs` covers the no-stack property.
