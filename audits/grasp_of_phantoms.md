## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/58/grasp-of-phantoms?utm_source=api
**Type line**: `Sorcery` — {3}{U}
**Oracle text**:
```
Put target creature on top of its owner's library.
Flashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Put target creature **on top of** its owner's library" — inserted at position
  0 of the owner's `library_order`, not appended to the bottom: PASS
- "its **owner's** library", so a stolen creature goes to its owner's: PASS
- A token put on top of a library ceases to exist (CR 704.5e): PASS
- Flashback {7}{U}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Top-of-library placement and the flashback: `cards_flashback.rs`, `cards_bounce.rs`
