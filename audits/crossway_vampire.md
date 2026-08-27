## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/135/crossway-vampire?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}{R}, 3/2
**Oracle text**:
```
When this creature enters, target creature can't block this turn.
```

**Status**: PASS

### Code issues
No issues found.

'target creature can't block this turn' — targeted, locked at trigger time, applied through the shared `CantBlockThisTurn` effect rather than a hand-rolled flag.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
