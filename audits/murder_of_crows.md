## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/70/murder-of-crows?utm_source=api
**Type line**: `Creature — Bird` — {3}{U}{U}, 4/4
**Oracle text**:
```
Flying
Whenever another creature dies, you may draw a card. If you do, discard a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **another** creature dies" — including an opponent's, and including
  tokens: PASS
- "you **may** draw a card. **If you do**, discard a card" — the discard is
  conditional on the draw, so declining costs nothing and an empty library that
  drew nothing does not force a discard: PASS
- The discard goes through `discard_card`, so it announces itself to discard
  watchers (Civilized Scholar's transform is in this set): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The may-draw and the linked discard: `cards_discard_and_hand.rs`, `simultaneous_events.rs`
