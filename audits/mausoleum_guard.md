## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/20/mausoleum-guard?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{W}, 2/2
**Oracle text**:
```
When this creature dies, create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying and their subtype, created for the last-known **controller** rather than the owner — so a stolen Guard's tokens go to whoever controlled it.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
