## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/72/runic-repetition?utm_source=api
**Type line**: `Sorcery` — {2}{U}
**Oracle text**:
```
Return target exiled card with flashback you own to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target exiled card **with flashback** **you own**" — all three: in exile,
  owned by the caster, and its card data declares a flashback cost: PASS
- CR 109.1 now keeps tokens out of the `ExileCard` enumeration engine-side: PASS
- The returned card's `cast_with_flashback` flag is cleared on the move, so it
  can be cast normally from hand and flashed back again later: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Returning an exiled flashback card: `cards_flashback.rs`
