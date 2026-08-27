## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/192/make-a-wish?utm_source=api
**Type line**: `Sorcery` — {3}{G}
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```
**Status**: PASS

### Code issues
No issues found.

- "Return two **cards** at random" — filters `!o.is_token` (CR 109.1) and
  excludes the spell itself, which is on the stack rather than in the graveyard
  while it resolves.
- Genuinely random via `shuffle`, and `take(2)` handles a graveyard with fewer
  than two cards without panicking.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/192/make-a-wish?utm_source=api
**Type line**: `Sorcery` — {3}{G}
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Return **two cards at random** from your graveyard to your hand" — random, not
  chosen, and the pick is made at resolution: PASS
- **Any** cards, not just creature cards — lands and spells alike: PASS
- CR 109.1: "two **cards**", so a token in the graveyard is not a candidate: PASS
- Make a Wish itself is still on the stack while it resolves, so it excludes its
  own id and cannot return itself: PASS
- A graveyard with one card returns that one rather than failing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The random return and the token exclusion: `cards_graveyard_recursion.rs`, `token_is_not_a_card.rs`
