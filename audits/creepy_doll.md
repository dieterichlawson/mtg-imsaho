## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/220/creepy-doll?utm_source=api
**Type line**: `Artifact Creature — Construct` — {5}, 1/1
**Oracle text**:
```
Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "flip a coin. **If you win the flip**, destroy that creature" — a real 50/50,
  not an auto-win: PASS
- "deals **combat** damage to a creature", so a Geistflame does not set it off:
  PASS
- Indestructible on the Doll itself, so it survives what it kills: PASS
- `try_destroy`, so an indestructible victim survives the flip: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The coin flip and the destroy: `cards_complex_creatures.rs`, `combat_rules.rs`
