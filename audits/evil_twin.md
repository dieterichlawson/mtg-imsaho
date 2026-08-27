## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/212/evil-twin?utm_source=api
**Type line**: `Creature — Shapeshifter` — {2}{U}{B}, 0/0
**Oracle text**:
```
You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You **may** have this creature enter as a copy" — declining leaves a
  0/0 that dies to state-based actions: PASS
- Ruling: "Evil Twin copies exactly what was printed on the original creature
  ... It doesn't copy whether that creature is tapped or untapped, whether it has
  any counters on it or any Auras and Equipment attached to it, or any non-copy
  effects that have changed its power, toughness, types, color": PASS
- Ruling: "If the chosen creature is copying something else ... your Evil Twin
  enters the battlefield as whatever the chosen creature copied": PASS
- Ruling: "The activated ability that Evil Twin gains as part of its copy effect
  is a copiable value" — the granted ability is dispatched through
  `copy_grantor` (CR 706.2), which is also how the engine resolves whose
  behavior an ability belongs to: PASS
- "Destroy target creature **with the same name as this creature**" — the name
  comes from the active face, not `obj.name`'s display cache: PASS
- Ruling: "If Evil Twin somehow enters the battlefield at the same time as
  another creature, Evil Twin can't become a copy of that creature": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The copy choice, declining, and the granted ability: `cards_complex_creatures.rs`, `subtype.rs`, `characteristics_targeting.rs`
