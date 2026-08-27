## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/83/think-twice?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Draws one card; the flashback cost is declared in card data and was verified exact set-wide.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/83/think-twice?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Draw a card; flashback {2}{U}. The draw goes through `engine::draw_cards`, so
an empty library is handled by the engine (and by Laboratory Maniac's
replacement) rather than by the card.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`flashback.rs` — cast from hand, then from the graveyard, then exiled.
