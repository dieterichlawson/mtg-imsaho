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


### Tricky interactions checked
- "Morbid — ... **if** a creature died this turn, you gain 5 life" is an
  intervening-if (CR 603.4), checked both when the trigger would go on the stack
  and again on resolution: PASS
- The Scavenger's own arrival cannot satisfy its condition — entering is not
  dying: PASS
- The life gain goes through `change_life`, so LifeChanged reaches every
  watcher: PASS
- CR 113.7a: killing the Scavenger in response to its own trigger does not stop
  the life gain: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The morbid condition and the life gain: `cards_morbid_and_ltb.rs`, `intervening_if.rs`
