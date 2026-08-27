## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/162/scourge-of-geier-reach?utm_source=api
**Type line**: `Creature — Elemental` — {3}{R}{R}, 3/3
**Oracle text**:
```
This creature gets +1/+1 for each creature your opponents control.
```
**Status**: PASS

### Code issues
No issues found.

- "gets +1/+1 for each creature your opponents control" — `dynamic_pt` returns
  `3 + N`, and since `dynamic_pt` supplies the *base* in `effective_power`,
  counters and anthems still layer on top correctly.
- Recomputed on every read, which is right for a characteristic-defining
  ability: the bonus tracks the opponent's board as it changes.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/162/scourge-of-geier-reach?utm_source=api
**Type line**: `Creature — Elemental` — {3}{R}{R}, 3/3
**Oracle text**:
```
This creature gets +1/+1 for each creature your opponents control.
```

**Status**: ISSUE (fixed)

### Code issues
See below.

- Oracle text says: `This creature gets +1/+1 for each creature your opponents control.`
  - Code did: `let opponent = state.opponent(controller); ... o.controller == opponent`
  - "your opponents" is everyone who is not you, not one named opponent. Not
    observable in a two-player game, but it encoded a two-player assumption
    into a card that does not have one. Changed to `o.controller != controller`,
    matching how Hysterical Blindness and the rest of the set read the same
    phrasing.

The count itself is a `dynamic_pt`, so it is recomputed as the board changes
rather than snapshotted (this is a characteristic-defining ability, not a
one-shot effect).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_spells_and_enchantments.rs::scourge_of_geier_reach_counts_only_opponents_creatures` — own creatures excluded, opponent's counted, recomputed on change.
