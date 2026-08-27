## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/15/fiend-hunter?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}{W}, 1/3
**Oracle text**:
```
When this creature enters, you may exile another target creature.
When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- CR 603.3d: "exile **another target** creature" is targeted, so the target is
  locked when the ETB trigger goes on the stack; only the "you may" decision
  remains at resolution, and the card offers exactly that locked target rather
  than a fresh pick: PASS
- "**another**" — it cannot exile itself: PASS
- "return the exiled card to the battlefield **under its owner's control**"
  (CR 110.2), and that is true when `EnteredBattlefield` fires rather than
  corrected afterwards: PASS
- The LTB trigger only returns a card still in exile, so a second effect that
  moved it in the meantime is respected (CR 608.2): PASS
- Exiling a token means it never comes back: PASS
- Declining the "you may" exiles nothing, and the LTB trigger then returns
  nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The exile/return pair and the locked target: `cards_complex_creatures.rs`, `trigger_target_recheck.rs`
- Returning under the owner's control: `control_change.rs`
