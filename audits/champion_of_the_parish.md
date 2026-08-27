## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/6/champion-of-the-parish?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
Whenever another Human you control enters, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**another** Human you control enters" — the engine's `AnyCreatureEnters`
  collector excludes the entering permanent from the watcher list, so the
  Champion never sees its own arrival; the entering permanent gets `SelfEntered`
  instead: PASS
- "you control": PASS
- `has_subtype` reads the ACTIVE face, so a transformed Werewolf no longer counts
  as a Human — a hand-rolled `registry.card_data` check would always have read
  the front face: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The subtype and controller filters: `subtype.rs`, `cards_complex_creatures.rs`
