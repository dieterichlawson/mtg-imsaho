## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a creature **dealt damage by this creature this turn** dies, you gain
  life equal to that creature's **toughness**" — reads the captured
  `dead_damaged_by` and `dead_toughness`, which is last-known information
  (CR 603.6d, cited in the code). A dead creature's toughness cannot be read off
  the object afterwards.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
