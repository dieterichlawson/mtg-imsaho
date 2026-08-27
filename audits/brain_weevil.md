## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/91/brain-weevil?utm_source=api
**Type line**: `Creature — Insect` — {3}{B}, 1/1
**Oracle text**:
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Sacrifice this creature**: Target player discards two cards" — the
  sacrifice is a cost, paid on activation, so the Weevil is in the graveyard
  while the ability is on the stack: PASS
- "**Activate only as a sorcery**" — `sorcery_speed_only`: PASS
- "discards **two** cards" — both, chained, and the discards go through
  `discard_card` so watchers see them: PASS
- A player with one card discards one and is not made to discard twice: PASS
- Intimidate is printed, not menace: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both discards and the sorcery-speed restriction: `auto_pick.rs:bug_brain_weevil_incomplete_discard`, `simultaneous_events.rs`
