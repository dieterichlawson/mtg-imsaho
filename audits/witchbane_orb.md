## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/236/witchbane-orb?utm_source=api
**Type line**: `Artifact` — {4}
**Oracle text**:
```
When this artifact enters, destroy all Curses attached to you.
You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "destroy **all** Curses attached to you" — every one, via `try_destroy_all`,
  and only those attached to the Orb's controller: PASS
- "You have hexproof" — a *player* hexproof, so opponents cannot target you
  with spells or abilities, including Aura spells (Curses): PASS
- The ETB destruction and the static hexproof are separate: an existing Curse
  is destroyed, and future ones cannot be attached: PASS
- Your own spells can still target you: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Curse sweep and player hexproof: `hexproof_filter.rs`, `player_protection.rs`
