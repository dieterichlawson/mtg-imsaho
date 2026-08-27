## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/166/traitorous-blood?utm_source=api
**Type line**: `Sorcery` — {1}{R}{R}
**Oracle text**:
```
Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Gain control of target creature **until end of turn**" — a temporary control
  change that reverts at cleanup, and reverts to the *original* controller
  rather than to its owner: PASS
- "**Untap** it. It gains trample and **haste**" — haste is what makes the
  stolen creature able to attack, and all three effects end together: PASS
- Control changing back does not untap or remove it from combat: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The temporary control change and the granted keywords: `control_durations.rs`, `control_change.rs`
