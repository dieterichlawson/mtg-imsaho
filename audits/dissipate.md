## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/53/dissipate?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2004-10-04]**: "If the spell is not countered (because the spell it
targets can't be countered), then it does not get exiled." And: "The card does
not go to the graveyard before being exiled."

- The exile is conditional on the counter actually happening, and the countered
  spell goes straight to exile rather than to the graveyard and then out.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
