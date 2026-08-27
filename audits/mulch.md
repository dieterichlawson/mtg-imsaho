## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/196/mulch?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```
**Status**: PASS

### Code issues
No issues found.

Reveals the top four, lands to hand and the rest to the graveyard, using `has_card_type(Land)`. Handles a library of fewer than four.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/196/mulch?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Put **all land cards** revealed this way into your hand and **the rest** into
  your graveyard" — lands to hand, everything else to the graveyard, with no
  choice: PASS
- The graveyard half is a library-to-graveyard move, so it goes through
  `mill_one` and a creature card among them emits `CreatureCardMilled`: PASS
- A library with fewer than four cards reveals what it has: PASS
- The land test reads the card's active face rather than the object's empty
  `card_types`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The split and the mill event: `multi_target_and_mill.rs:mulch_emits_creature_card_milled`
