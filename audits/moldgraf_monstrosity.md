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


### Tricky interactions checked
- "When this creature dies, **exile it**, then return two creature cards **at
  random** from your graveyard to the battlefield" — the exile is of itself, and
  the return is random rather than chosen: PASS
- CR 608.2: an ability does as much as it can — the return still happens if the
  Monstrosity was already exiled from the graveyard in response: PASS
- Two Monstrosities dying together: the first trigger can return the second, and
  the second trigger must then leave it on the battlefield rather than exiling a
  live creature: PASS
- CR 109.1: "two creature **cards** at random", so a token in the graveyard is
  not a candidate: PASS
- Trample: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both orderings and the token exclusion: `resolution_time_checks.rs:moldgraf_simultaneous_death_second_trigger_does_not_exile_live_creature`, `:moldgraf_exile_skipped_when_already_exiled_still_returns_creatures`, `token_is_not_a_card.rs:moldgraf_does_not_return_a_token_from_the_graveyard`
