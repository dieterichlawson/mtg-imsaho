## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/11/doomed-traveler?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
When this creature dies, create a 1/1 white Spirit creature token with flying.
```

**Status**: PASS

### Code issues
No issues found.

One 1/1 white Spirit token with flying, with its subtype set via `create_token_with_subtypes`.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/11/doomed-traveler?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
When this creature dies, create a 1/1 white Spirit creature token with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature dies, create a 1/1 white Spirit creature token **with
  flying**": PASS
- It triggers on any death — sacrificed, destroyed, or lethal damage: PASS
- Exiling it instead of letting it die gives no token: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Spirit token on death: `cards_morbid_and_ltb.rs`
