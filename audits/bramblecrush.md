## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/172/bramblecrush?utm_source=api
**Type line**: `Sorcery` — {2}{G}{G}
**Oracle text**:
```
Destroy target noncreature permanent.
```

**Status**: PASS

### Code issues
No issues found.

- "Destroy target **noncreature** permanent" — `is_valid_target` requires the
  permanent's face **not** to include Creature, so an animated permanent or a
  creature is excluded.
- Destroys through the pipeline, so indestructible applies; the oracle says
  destroy, not exile.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
