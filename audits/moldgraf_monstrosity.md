## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/194/moldgraf-monstrosity?utm_source=api
**Type line**: `Creature — Insect` — {4}{G}{G}{G}, 8/8
**Oracle text**:
```
Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
```

**Status**: PASS

### Code issues
No issues found.

- "When this creature dies, **exile it**, then return two creature **cards** at
  random from your graveyard to the battlefield" — the exile applies to the card
  in the graveyard and only there, and the code comments the ordering hazard:
  two Monstrosities dying together each put a trigger on the stack, and the first
  can return the second.
- "your graveyard" uses the last-known **controller**, not the owner (CR 603.10c),
  which matters for a stolen Monstrosity.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
