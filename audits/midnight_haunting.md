## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/22/midnight-haunting?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Create two 1/1 white Spirit creature tokens with flying.
```
**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying, created with their subtype. Instant speed comes from the card type, not a Flash keyword.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/22/midnight-haunting?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying, same per-token creation as Moan of
the Unhallowed. An instant — castable at end of turn or as a combat trick, which
is the whole point of the card over Moan.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs` — count, flying, and Spirit subtype for Geist-Honored Monk interactions.
