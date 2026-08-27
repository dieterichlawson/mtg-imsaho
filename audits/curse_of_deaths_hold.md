## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/94/curse-of-deaths-hold?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}{B}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls get -1/-1.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Creatures enchanted player controls get -1/-1" — a *static* ability, so it
  applies to creatures that arrive later too, unlike a spell's anthem
  (CR 611.2c). `EffectScope::Global(ControlledByAttachedPlayer)` is re-evaluated
  rather than snapshotted: PASS
- A 1/1 the cursed player controls dies to state-based actions: PASS
- It follows control changes — the filter is "controlled by", read live: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The debuff and its scope: `curse_and_equip_scope.rs`, `snapshot_anthems.rs:a_static_anthem_stops_when_its_source_leaves`
