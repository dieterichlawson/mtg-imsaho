## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/111/night-terrors?utm_source=api
**Type line**: `Sorcery` — {2}{B}
**Oracle text**:
```
Target player reveals their hand. You choose a nonland card from it. Exile that card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**You** choose a nonland card from it" — the chooser is the spell's
  controller, not the targeted player: PASS
- "a **nonland** card" — lands in hand are not offered, read from the card's
  active face rather than the object's empty `card_types`: PASS
- Exile, not discard, so it does not trigger discard watchers (Murder of Crows
  is in this set): PASS
- A hand with no nonland card resolves with no effect: PASS
- Ruling: "If you target yourself with this spell, you must reveal your entire
  hand" — targeting yourself is legal: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The choice and the exile: `cards_discard_and_hand.rs`
