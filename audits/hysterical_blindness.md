## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/59/hysterical-blindness?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```
**Status**: PASS

### Code issues
No issues found.

- "Creatures your opponents control get -4/-0 until end of turn" — snapshots the
  affected creatures at resolution and pushes one per-target `ModifyPT`, which is
  what CR 611.2c requires of a spell. This card had it right where four others in
  the set did not.
- -4/-0, not -4/-4: power only.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/59/hysterical-blindness?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Creatures your opponents control get -4/-0 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.

Two rulings, both satisfied. "Only creatures controlled by your opponent when
[it] resolves will get -4/-0" — the ids are collected into a `Vec` at resolve
time. "The effect will continue to apply to a creature even if you ... gain
control of that creature later in the turn" — the effect is
`ModifyPT { target: id }`, keyed on the object, so a control change does not
detach it.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`snapshot_anthems.rs` — the set is fixed at resolution.
