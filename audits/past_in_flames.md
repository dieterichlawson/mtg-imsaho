## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/155/past-in-flames?utm_source=api
**Type line**: `Sorcery` — {3}{R}
**Oracle text**:
```
Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "Each instant and sorcery **card** in your graveyard" — filters `!o.is_token`
  and reads types from the card's face.
- CR 702.33a: the granted flashback cost equals the card's mana cost, so a card
  with no mana cost is skipped rather than handed a free one — covered by
  `flashback_multiple_instances.rs`.
- Excludes itself, which is on the stack while resolving.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/155/past-in-flames?utm_source=api
**Type line**: `Sorcery` — {3}{R}
**Oracle text**:
```
Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Past in Flames affects **only cards in your graveyard at the time it
  resolves**. Instant and sorcery cards put into your graveyard later in the turn
  won't gain flashback." The list is built at resolution: PASS
- CR 702.33a: "The flashback cost is equal to its **mana cost**" — a card with no
  mana cost is skipped rather than given a free flashback: PASS
- Past in Flames is still on the stack while it resolves, so it is not in its own
  list — and the engine moves it afterwards (CR 608.2m): PASS
- CR 109.1: "each instant and sorcery **card**", so tokens are excluded: PASS
- "in **your** graveyard": PASS
- Its own flashback {4}{R} is separate from the flashback it grants: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Granting flashback at resolution: `cards_flashback.rs`
