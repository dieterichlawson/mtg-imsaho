## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/75/silent-departure?utm_source=api
**Type line**: `Sorcery` — {U}
**Oracle text**:
```
Return target creature to its owner's hand.
Flashback {4}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "to its **owner's** hand" — the hand zone is keyed by owner, so a stolen
  creature goes back to its owner rather than its controller: PASS
- A token returned to hand ceases to exist (CR 704.5e): PASS
- Auras and Equipment attached to it fall off (CR 704.5m / detach): PASS
- Flashback {4}{U}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The bounce and the flashback: `cards_flashback.rs`, `cards_bounce.rs`
