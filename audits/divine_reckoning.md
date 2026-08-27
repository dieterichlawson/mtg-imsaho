## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/10/divine-reckoning?utm_source=api
**Type line**: `Sorcery` — {2}{W}{W}
**Oracle text**:
```
Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Each player** chooses a creature they control. Destroy the rest." — every
  player chooses, in turn order, and nothing is destroyed until all have chosen:
  PASS
- A player with no creatures chooses nothing and loses nothing: PASS
- The choices are collected through a chained `resolve_card_effect` that encodes
  who has already chosen, and the spell stays on the stack throughout — this is
  the card the CR 608.2m rule was written for, that reaching the graveyard is
  the *final* step of resolution: PASS
- `try_destroy_all`, so the rest die simultaneously and indestructible survives:
  PASS
- Flashback {5}{W}{W}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The per-player choice chain and the simultaneous destruction: `spell_cleanup.rs`, `cards_flashback.rs`
