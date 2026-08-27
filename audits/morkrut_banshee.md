## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/110/morkrut-banshee?utm_source=api
**Type line**: `Creature — Spirit` — {3}{B}{B}, 4/4
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — ... **if** a creature died this turn" is an intervening-if
  (CR 603.4): checked when the trigger would go on the stack *and* again on
  resolution, via `should_trigger`: PASS
- -4/-4 until end of turn kills a 4/4 by state-based action, so indestructible
  does not save it: PASS
- The trigger is targeted, so it is not put on the stack at all with no legal
  creature to point at: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Morbid as an intervening-if: `intervening_if.rs`, `cards_morbid_and_ltb.rs`
