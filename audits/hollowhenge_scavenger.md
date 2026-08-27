## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/188/hollowhenge-scavenger?utm_source=api
**Type line**: `Creature — Elemental` — {3}{G}{G}, 4/5
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
```

**Status**: PASS

### Code issues
No issues found.

- Same morbid intervening-if shape as Woodland Sleuth, gated at dispatch and
  re-checked at resolution.
- Emits `LifeChanged` for the 5 life.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
